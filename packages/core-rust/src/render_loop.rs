//! Render loop with FPS-capped frame scheduling and dirty tracking.
//!
//! Implements the Layout -> Render -> Diff -> Output pipeline with
//! request-based rendering: components mark themselves dirty, and the
//! next frame picks them up. Only dirty subtrees are re-laid-out and
//! re-rendered, minimising work per frame.

use std::collections::HashSet;
use std::io::Write;
use std::time::{Duration, Instant};

use crate::cell::{CellBuffer, DoubleBuffer};
use crate::layout::{ComputedLayout, LayoutNodeId, LayoutTree, NodeStyle};

// ---------------------------------------------------------------------------
// Frame metrics
// ---------------------------------------------------------------------------

/// Timing information for a single frame.
#[derive(Debug, Clone, Copy)]
pub struct FrameMetrics {
    /// Sequence number (monotonically increasing).
    pub frame_number: u64,
    /// Time spent computing layout.
    pub layout_duration: Duration,
    /// Time spent rendering cells into the back buffer.
    pub render_duration: Duration,
    /// Time spent diffing front/back buffers.
    pub diff_duration: Duration,
    /// Time spent writing output.
    pub output_duration: Duration,
    /// Total frame time (layout + render + diff + output).
    pub total_duration: Duration,
    /// Number of dirty nodes processed this frame.
    pub dirty_node_count: usize,
    /// Number of bytes written to output.
    pub output_bytes: usize,
}

// ---------------------------------------------------------------------------
// Render node — ties a layout node to its render callback
// ---------------------------------------------------------------------------

/// A callback that renders a component into a region of the cell buffer.
///
/// Receives the computed layout (position + size in cell coordinates)
/// and a mutable reference to the back buffer.
pub type RenderFn = Box<dyn FnMut(&ComputedLayout, &mut CellBuffer)>;

/// A node in the render tree: layout identity + optional render callback.
struct RenderNode {
    layout_id: LayoutNodeId,
    style: NodeStyle,
    render_fn: Option<RenderFn>,
    children: Vec<RenderNodeId>,
    parent: Option<RenderNodeId>,
}

/// Handle to a render node.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RenderNodeId(usize);

// ---------------------------------------------------------------------------
// Render loop configuration
// ---------------------------------------------------------------------------

/// Configuration for the render loop.
#[derive(Debug, Clone)]
pub struct RenderLoopConfig {
    /// Target frames per second. 0 means unlimited.
    pub target_fps: u32,
    /// Terminal width in columns.
    pub width: usize,
    /// Terminal height in rows.
    pub height: usize,
}

impl Default for RenderLoopConfig {
    fn default() -> Self {
        Self {
            target_fps: 30,
            width: 80,
            height: 24,
        }
    }
}

// ---------------------------------------------------------------------------
// RenderLoop
// ---------------------------------------------------------------------------

/// The main render loop coordinating layout, rendering, diffing, and output.
pub struct RenderLoop {
    config: RenderLoopConfig,
    layout_tree: LayoutTree,
    double_buffer: DoubleBuffer,
    nodes: Vec<RenderNode>,
    root: Option<RenderNodeId>,
    dirty: HashSet<RenderNodeId>,
    frame_number: u64,
    last_metrics: Option<FrameMetrics>,
    needs_full_redraw: bool,
}

impl RenderLoop {
    /// Create a new render loop with the given configuration.
    #[must_use]
    pub fn new(config: RenderLoopConfig) -> Self {
        Self {
            double_buffer: DoubleBuffer::new(config.width, config.height),
            layout_tree: LayoutTree::new(),
            config,
            nodes: Vec::new(),
            root: None,
            dirty: HashSet::new(),
            frame_number: 0,
            last_metrics: None,
            needs_full_redraw: true,
        }
    }

    /// Return the frame interval based on `target_fps`.
    /// Returns `None` if fps is 0 (unlimited).
    #[must_use]
    pub fn frame_interval(&self) -> Option<Duration> {
        if self.config.target_fps == 0 {
            None
        } else {
            Some(Duration::from_secs_f64(
                1.0 / f64::from(self.config.target_fps),
            ))
        }
    }

    /// Get the current target FPS.
    #[must_use]
    pub fn target_fps(&self) -> u32 {
        self.config.target_fps
    }

    /// Set the target FPS. Pass 0 for unlimited.
    pub fn set_target_fps(&mut self, fps: u32) {
        self.config.target_fps = fps;
    }

    /// Add a leaf render node (no children) with an optional render callback.
    ///
    /// # Errors
    ///
    /// Returns an error if the layout node cannot be created.
    pub fn add_leaf(
        &mut self,
        style: NodeStyle,
        render_fn: Option<RenderFn>,
    ) -> Result<RenderNodeId, RenderLoopError> {
        let layout_id = self
            .layout_tree
            .add_leaf(&style)
            .map_err(|_| RenderLoopError::LayoutError)?;
        let id = RenderNodeId(self.nodes.len());
        self.nodes.push(RenderNode {
            layout_id,
            style,
            render_fn,
            children: Vec::new(),
            parent: None,
        });
        self.dirty.insert(id);
        Ok(id)
    }

    /// Add a container render node with children.
    ///
    /// # Errors
    ///
    /// Returns an error if the layout node cannot be created.
    pub fn add_node(
        &mut self,
        style: NodeStyle,
        children: &[RenderNodeId],
        render_fn: Option<RenderFn>,
    ) -> Result<RenderNodeId, RenderLoopError> {
        let layout_children: Vec<LayoutNodeId> =
            children.iter().map(|c| self.nodes[c.0].layout_id).collect();
        let layout_id = self
            .layout_tree
            .add_node(&style, &layout_children)
            .map_err(|_| RenderLoopError::LayoutError)?;
        let id = RenderNodeId(self.nodes.len());
        for &child in children {
            self.nodes[child.0].parent = Some(id);
        }
        self.nodes.push(RenderNode {
            layout_id,
            style,
            render_fn,
            children: children.to_vec(),
            parent: None,
        });
        self.dirty.insert(id);
        Ok(id)
    }

    /// Set the root node for the render tree.
    pub fn set_root(&mut self, root: RenderNodeId) {
        self.root = Some(root);
        self.needs_full_redraw = true;
    }

    /// Mark a node as dirty so it will be re-rendered next frame.
    pub fn mark_dirty(&mut self, node: RenderNodeId) {
        self.dirty.insert(node);
    }

    /// Update the style of an existing node and mark it dirty.
    ///
    /// # Errors
    ///
    /// Returns an error if the layout update fails.
    pub fn update_style(
        &mut self,
        node: RenderNodeId,
        style: NodeStyle,
    ) -> Result<(), RenderLoopError> {
        let layout_id = self.nodes[node.0].layout_id;
        self.layout_tree
            .set_style(layout_id, &style)
            .map_err(|_| RenderLoopError::LayoutError)?;
        self.nodes[node.0].style = style;
        self.mark_dirty(node);
        Ok(())
    }

    /// Check whether any nodes are dirty (a frame is needed).
    #[must_use]
    pub fn needs_render(&self) -> bool {
        self.needs_full_redraw || !self.dirty.is_empty()
    }

    /// Resize the terminal dimensions. Triggers a full redraw.
    pub fn resize(&mut self, width: usize, height: usize) {
        self.config.width = width;
        self.config.height = height;
        self.double_buffer.resize(width, height);
        self.needs_full_redraw = true;
    }

    /// Request a full redraw on the next frame (e.g. after terminal corruption).
    pub fn request_full_redraw(&mut self) {
        self.needs_full_redraw = true;
    }

    /// Get the metrics from the last rendered frame.
    #[must_use]
    pub fn last_metrics(&self) -> Option<&FrameMetrics> {
        self.last_metrics.as_ref()
    }

    /// Get the current frame number.
    #[must_use]
    pub fn frame_number(&self) -> u64 {
        self.frame_number
    }

    /// Run a single frame: layout -> render -> diff -> output.
    ///
    /// Writes ANSI output to `writer`. Returns the frame metrics.
    ///
    /// # Errors
    ///
    /// Returns an error if layout computation or output writing fails.
    #[allow(clippy::cast_precision_loss)] // terminal dimensions are small
    pub fn run_frame<W: Write>(&mut self, writer: &mut W) -> Result<FrameMetrics, RenderLoopError> {
        let frame_start = Instant::now();

        let root = self.root.ok_or(RenderLoopError::NoRoot)?;
        let root_layout_id = self.nodes[root.0].layout_id;

        // --- Layout phase ---
        let layout_start = Instant::now();
        let dirty_count = if self.needs_full_redraw {
            // Full re-layout
            self.layout_tree
                .compute(
                    root_layout_id,
                    self.config.width as f32,
                    self.config.height as f32,
                )
                .map_err(|_| RenderLoopError::LayoutError)?;
            self.nodes.len()
        } else if !self.dirty.is_empty() {
            // Even with partial dirty flags, Taffy requires recomputing
            // from the root (it handles internal caching).
            self.layout_tree
                .compute(
                    root_layout_id,
                    self.config.width as f32,
                    self.config.height as f32,
                )
                .map_err(|_| RenderLoopError::LayoutError)?;
            self.dirty.len()
        } else {
            0
        };
        let layout_duration = layout_start.elapsed();

        // --- Render phase ---
        let render_start = Instant::now();
        let back = self.double_buffer.back_mut();
        back.clear();

        // Collect node indices that need rendering.
        let nodes_to_render: Vec<usize> = if self.needs_full_redraw {
            (0..self.nodes.len()).collect()
        } else {
            self.collect_dirty_subtree()
        };

        // Render each dirty node that has a render_fn.
        for idx in &nodes_to_render {
            let layout_id = self.nodes[*idx].layout_id;
            if let Ok(layout) = self.layout_tree.get_layout(layout_id) {
                if self.nodes[*idx].render_fn.is_some() {
                    // Take the render_fn out temporarily to satisfy borrow checker.
                    let mut render_fn = self.nodes[*idx].render_fn.take();
                    if let Some(ref mut f) = render_fn {
                        f(&layout, self.double_buffer.back_mut());
                    }
                    self.nodes[*idx].render_fn = render_fn;
                }
            }
        }
        let render_duration = render_start.elapsed();

        // --- Diff phase ---
        let diff_start = Instant::now();
        let output_bytes = if self.needs_full_redraw {
            self.double_buffer.full_render()
        } else {
            self.double_buffer.diff()
        };
        let diff_duration = diff_start.elapsed();

        // --- Output phase ---
        let output_start = Instant::now();
        let bytes_len = output_bytes.len();
        if !output_bytes.is_empty() {
            writer
                .write_all(&output_bytes)
                .map_err(|e| RenderLoopError::IoError(e.to_string()))?;
            writer
                .flush()
                .map_err(|e| RenderLoopError::IoError(e.to_string()))?;
        }
        let output_duration = output_start.elapsed();

        // --- Bookkeeping ---
        self.double_buffer.swap_no_clear();
        self.dirty.clear();
        self.needs_full_redraw = false;
        self.frame_number += 1;

        let metrics = FrameMetrics {
            frame_number: self.frame_number,
            layout_duration,
            render_duration,
            diff_duration,
            output_duration,
            total_duration: frame_start.elapsed(),
            dirty_node_count: dirty_count,
            output_bytes: bytes_len,
        };
        self.last_metrics = Some(metrics);
        Ok(metrics)
    }

    /// Collect all node indices in dirty subtrees.
    /// When a node is dirty, all its descendants should also be rendered.
    fn collect_dirty_subtree(&self) -> Vec<usize> {
        let mut result = HashSet::new();
        for &dirty_id in &self.dirty {
            self.collect_subtree(dirty_id, &mut result);
        }
        let mut sorted: Vec<usize> = result.into_iter().collect();
        sorted.sort_unstable();
        sorted
    }

    fn collect_subtree(&self, node: RenderNodeId, result: &mut HashSet<usize>) {
        if !result.insert(node.0) {
            return;
        }
        for &child in &self.nodes[node.0].children {
            self.collect_subtree(child, result);
        }
    }

    /// Wait until the next frame is due based on FPS cap.
    /// Returns the time remaining until the next frame (zero if already due).
    #[must_use]
    pub fn time_until_next_frame(&self, last_frame_time: Instant) -> Duration {
        match self.frame_interval() {
            None => Duration::ZERO,
            Some(interval) => {
                let elapsed = last_frame_time.elapsed();
                interval.saturating_sub(elapsed)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors from the render loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderLoopError {
    /// No root node has been set.
    NoRoot,
    /// Layout computation failed.
    LayoutError,
    /// IO error during output.
    IoError(String),
}

impl std::fmt::Display for RenderLoopError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoRoot => write!(f, "no root node set"),
            Self::LayoutError => write!(f, "layout computation failed"),
            Self::IoError(msg) => write!(f, "IO error: {msg}"),
        }
    }
}

impl std::error::Error for RenderLoopError {}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ansi::Style;
    use crate::layout::{Dim, DisplayMode, FlexDir, FlexStyle};

    fn default_config() -> RenderLoopConfig {
        RenderLoopConfig {
            target_fps: 30,
            width: 20,
            height: 10,
        }
    }

    // -- Configuration and frame interval --

    #[test]
    fn default_config_30fps() {
        let config = RenderLoopConfig::default();
        assert_eq!(config.target_fps, 30);
        assert_eq!(config.width, 80);
        assert_eq!(config.height, 24);
    }

    #[test]
    fn frame_interval_30fps() {
        let rl = RenderLoop::new(RenderLoopConfig {
            target_fps: 30,
            ..default_config()
        });
        let interval = rl.frame_interval().expect("should have interval");
        let expected = Duration::from_secs_f64(1.0 / 30.0);
        let diff = if interval > expected {
            interval - expected
        } else {
            expected - interval
        };
        assert!(diff < Duration::from_micros(100));
    }

    #[test]
    fn frame_interval_60fps() {
        let rl = RenderLoop::new(RenderLoopConfig {
            target_fps: 60,
            ..default_config()
        });
        let interval = rl.frame_interval().expect("should have interval");
        let expected = Duration::from_secs_f64(1.0 / 60.0);
        let diff = if interval > expected {
            interval - expected
        } else {
            expected - interval
        };
        assert!(diff < Duration::from_micros(100));
    }

    #[test]
    fn frame_interval_unlimited() {
        let rl = RenderLoop::new(RenderLoopConfig {
            target_fps: 0,
            ..default_config()
        });
        assert!(rl.frame_interval().is_none());
    }

    #[test]
    fn set_target_fps() {
        let mut rl = RenderLoop::new(default_config());
        assert_eq!(rl.target_fps(), 30);
        rl.set_target_fps(60);
        assert_eq!(rl.target_fps(), 60);
    }

    // -- Node management --

    #[test]
    fn add_leaf_creates_node() {
        let mut rl = RenderLoop::new(default_config());
        let style = NodeStyle {
            width: Dim::Cells(10.0),
            height: Dim::Cells(5.0),
            ..NodeStyle::default()
        };
        let id = rl.add_leaf(style, None).expect("should create leaf");
        assert_eq!(id, RenderNodeId(0));
    }

    #[test]
    fn add_node_with_children() {
        let mut rl = RenderLoop::new(default_config());
        let leaf_style = NodeStyle {
            width: Dim::Cells(10.0),
            height: Dim::Cells(5.0),
            ..NodeStyle::default()
        };
        let a = rl.add_leaf(leaf_style.clone(), None).expect("leaf a");
        let b = rl.add_leaf(leaf_style, None).expect("leaf b");

        let parent_style = NodeStyle {
            width: Dim::Cells(20.0),
            height: Dim::Cells(10.0),
            ..NodeStyle::default()
        };
        let parent = rl
            .add_node(parent_style, &[a, b], None)
            .expect("parent node");
        assert_eq!(parent, RenderNodeId(2));
    }

    #[test]
    fn mark_dirty_flags_node() {
        let mut rl = RenderLoop::new(default_config());
        let style = NodeStyle::default();
        let id = rl.add_leaf(style, None).expect("leaf");
        // Clear dirty from creation and initial full redraw flag
        rl.dirty.clear();
        rl.needs_full_redraw = false;
        assert!(!rl.needs_render());

        rl.mark_dirty(id);
        assert!(rl.needs_render());
    }

    // -- Error handling --

    #[test]
    fn run_frame_without_root_errors() {
        let mut rl = RenderLoop::new(default_config());
        let mut output = Vec::new();
        let result = rl.run_frame(&mut output);
        assert_eq!(result.unwrap_err(), RenderLoopError::NoRoot);
    }

    // -- Full frame pipeline --

    #[test]
    fn run_frame_first_frame_full_redraw() {
        let mut rl = RenderLoop::new(default_config());
        let style = NodeStyle {
            width: Dim::Cells(20.0),
            height: Dim::Cells(10.0),
            ..NodeStyle::default()
        };
        let root = rl
            .add_leaf(
                style,
                Some(Box::new(|layout, buf| {
                    let col = layout.x as usize;
                    let row = layout.y as usize;
                    buf.put_str(row, col, "Hello", Style::new());
                })),
            )
            .expect("root");
        rl.set_root(root);

        let mut output = Vec::new();
        let metrics = rl.run_frame(&mut output).expect("frame should succeed");

        assert_eq!(metrics.frame_number, 1);
        assert!(metrics.output_bytes > 0);
        assert!(metrics.dirty_node_count > 0);
        // Output should contain "Hello"
        let output_str = String::from_utf8_lossy(&output);
        assert!(
            output_str.contains("Hello"),
            "output should contain Hello, got: {output_str}"
        );
    }

    #[test]
    fn run_frame_no_dirty_produces_no_output() {
        let mut rl = RenderLoop::new(default_config());
        let style = NodeStyle {
            width: Dim::Cells(20.0),
            height: Dim::Cells(10.0),
            ..NodeStyle::default()
        };
        let root = rl.add_leaf(style, None).expect("root");
        rl.set_root(root);

        // First frame: full redraw
        let mut output = Vec::new();
        rl.run_frame(&mut output).expect("frame 1");

        // Second frame: nothing dirty
        let mut output2 = Vec::new();
        let metrics = rl.run_frame(&mut output2).expect("frame 2");
        assert_eq!(metrics.output_bytes, 0);
    }

    #[test]
    fn dirty_node_triggers_render() {
        let mut rl = RenderLoop::new(default_config());
        let call_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let counter = call_count.clone();

        let style = NodeStyle {
            width: Dim::Cells(20.0),
            height: Dim::Cells(10.0),
            ..NodeStyle::default()
        };
        let root = rl
            .add_leaf(
                style,
                Some(Box::new(move |_layout, _buf| {
                    counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                })),
            )
            .expect("root");
        rl.set_root(root);

        // Frame 1
        let mut output = Vec::new();
        rl.run_frame(&mut output).expect("frame 1");
        assert_eq!(call_count.load(std::sync::atomic::Ordering::Relaxed), 1);

        // Frame 2: no dirty -> render_fn not called
        rl.run_frame(&mut output).expect("frame 2");
        assert_eq!(call_count.load(std::sync::atomic::Ordering::Relaxed), 1);

        // Mark dirty, frame 3
        rl.mark_dirty(root);
        rl.run_frame(&mut output).expect("frame 3");
        assert_eq!(call_count.load(std::sync::atomic::Ordering::Relaxed), 2);
    }

    #[test]
    fn update_style_marks_dirty() {
        let mut rl = RenderLoop::new(default_config());
        let style = NodeStyle {
            width: Dim::Cells(10.0),
            height: Dim::Cells(5.0),
            ..NodeStyle::default()
        };
        let root = rl.add_leaf(style, None).expect("root");
        rl.set_root(root);

        let mut output = Vec::new();
        rl.run_frame(&mut output).expect("frame 1");
        assert!(!rl.needs_render());

        let new_style = NodeStyle {
            width: Dim::Cells(15.0),
            height: Dim::Cells(5.0),
            ..NodeStyle::default()
        };
        rl.update_style(root, new_style).expect("update style");
        assert!(rl.needs_render());
    }

    // -- Resize --

    #[test]
    fn resize_triggers_full_redraw() {
        let mut rl = RenderLoop::new(default_config());
        let style = NodeStyle {
            width: Dim::Cells(20.0),
            height: Dim::Cells(10.0),
            ..NodeStyle::default()
        };
        let root = rl.add_leaf(style, None).expect("root");
        rl.set_root(root);

        let mut output = Vec::new();
        rl.run_frame(&mut output).expect("frame 1");
        assert!(!rl.needs_render());

        rl.resize(40, 20);
        assert!(rl.needs_render());
        assert!(rl.needs_full_redraw);
    }

    // -- Frame metrics --

    #[test]
    fn frame_metrics_accessible() {
        let mut rl = RenderLoop::new(default_config());
        assert!(rl.last_metrics().is_none());

        let style = NodeStyle {
            width: Dim::Cells(20.0),
            height: Dim::Cells(10.0),
            ..NodeStyle::default()
        };
        let root = rl.add_leaf(style, None).expect("root");
        rl.set_root(root);

        let mut output = Vec::new();
        rl.run_frame(&mut output).expect("frame");

        let metrics = rl.last_metrics().expect("should have metrics");
        assert_eq!(metrics.frame_number, 1);
        assert!(metrics.total_duration >= metrics.layout_duration);
    }

    #[test]
    fn frame_number_increments() {
        let mut rl = RenderLoop::new(default_config());
        let style = NodeStyle {
            width: Dim::Cells(20.0),
            height: Dim::Cells(10.0),
            ..NodeStyle::default()
        };
        let root = rl.add_leaf(style, None).expect("root");
        rl.set_root(root);

        assert_eq!(rl.frame_number(), 0);
        let mut output = Vec::new();
        rl.run_frame(&mut output).expect("frame 1");
        assert_eq!(rl.frame_number(), 1);
        rl.mark_dirty(root);
        rl.run_frame(&mut output).expect("frame 2");
        assert_eq!(rl.frame_number(), 2);
    }

    // -- Frame timing / FPS cap --

    #[test]
    fn time_until_next_frame_returns_remaining() {
        let rl = RenderLoop::new(RenderLoopConfig {
            target_fps: 30,
            ..default_config()
        });
        let now = Instant::now();
        let remaining = rl.time_until_next_frame(now);
        // Just created, so remaining should be close to 1/30s
        let interval = Duration::from_secs_f64(1.0 / 30.0);
        assert!(remaining <= interval);
    }

    #[test]
    fn time_until_next_frame_unlimited_is_zero() {
        let rl = RenderLoop::new(RenderLoopConfig {
            target_fps: 0,
            ..default_config()
        });
        let now = Instant::now();
        assert_eq!(rl.time_until_next_frame(now), Duration::ZERO);
    }

    // -- Dirty subtree propagation --

    #[test]
    fn dirty_parent_renders_children() {
        let mut rl = RenderLoop::new(default_config());
        let child_counter = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let counter = child_counter.clone();

        let child_style = NodeStyle {
            width: Dim::Cells(10.0),
            height: Dim::Cells(5.0),
            ..NodeStyle::default()
        };
        let child = rl
            .add_leaf(
                child_style,
                Some(Box::new(move |_layout, _buf| {
                    counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                })),
            )
            .expect("child");

        let parent_style = NodeStyle {
            display: DisplayMode::Flex(FlexStyle {
                direction: FlexDir::Column,
                ..FlexStyle::default()
            }),
            width: Dim::Cells(20.0),
            height: Dim::Cells(10.0),
            ..NodeStyle::default()
        };
        let parent = rl.add_node(parent_style, &[child], None).expect("parent");
        rl.set_root(parent);

        // Frame 1: full redraw
        let mut output = Vec::new();
        rl.run_frame(&mut output).expect("frame 1");
        assert_eq!(child_counter.load(std::sync::atomic::Ordering::Relaxed), 1);

        // Mark only parent dirty -> child should also render
        rl.mark_dirty(parent);
        rl.run_frame(&mut output).expect("frame 2");
        assert_eq!(child_counter.load(std::sync::atomic::Ordering::Relaxed), 2);
    }

    // -- Layout -> Render pipeline integration --

    #[test]
    fn layout_positions_are_passed_to_render_fn() {
        let mut rl = RenderLoop::new(RenderLoopConfig {
            width: 40,
            height: 20,
            ..default_config()
        });
        let received_layout = std::sync::Arc::new(std::sync::Mutex::new(None));
        let layout_ref = received_layout.clone();

        let style = NodeStyle {
            width: Dim::Cells(15.0),
            height: Dim::Cells(8.0),
            ..NodeStyle::default()
        };
        let root = rl
            .add_leaf(
                style,
                Some(Box::new(move |layout, _buf| {
                    if let Ok(mut guard) = layout_ref.lock() {
                        *guard = Some(*layout);
                    }
                })),
            )
            .expect("root");
        rl.set_root(root);

        let mut output = Vec::new();
        rl.run_frame(&mut output).expect("frame");

        let layout = received_layout
            .lock()
            .expect("lock")
            .expect("should have layout");
        assert!((layout.width - 15.0).abs() < f32::EPSILON);
        assert!((layout.height - 8.0).abs() < f32::EPSILON);
    }

    // -- Multiple nodes rendering --

    #[test]
    fn multiple_nodes_all_render() {
        let mut rl = RenderLoop::new(default_config());

        let child_a_style = NodeStyle {
            width: Dim::Cells(10.0),
            height: Dim::Cells(5.0),
            ..NodeStyle::default()
        };
        let child_a = rl
            .add_leaf(
                child_a_style,
                Some(Box::new(|layout, buf| {
                    buf.put_str(layout.y as usize, layout.x as usize, "AAA", Style::new());
                })),
            )
            .expect("child a");

        let child_b_style = NodeStyle {
            width: Dim::Cells(10.0),
            height: Dim::Cells(5.0),
            ..NodeStyle::default()
        };
        let child_b = rl
            .add_leaf(
                child_b_style,
                Some(Box::new(|layout, buf| {
                    buf.put_str(layout.y as usize, layout.x as usize, "BBB", Style::new());
                })),
            )
            .expect("child b");

        let parent_style = NodeStyle {
            display: DisplayMode::Flex(FlexStyle {
                direction: FlexDir::Row,
                ..FlexStyle::default()
            }),
            width: Dim::Cells(20.0),
            height: Dim::Cells(10.0),
            ..NodeStyle::default()
        };
        let parent = rl
            .add_node(parent_style, &[child_a, child_b], None)
            .expect("parent");
        rl.set_root(parent);

        let mut output = Vec::new();
        rl.run_frame(&mut output).expect("frame");

        let output_str = String::from_utf8_lossy(&output);
        assert!(
            output_str.contains("AAA"),
            "should contain AAA: {output_str}"
        );
        assert!(
            output_str.contains("BBB"),
            "should contain BBB: {output_str}"
        );
    }

    // -- Request full redraw --

    #[test]
    fn request_full_redraw_forces_full_output() {
        let mut rl = RenderLoop::new(default_config());
        let style = NodeStyle {
            width: Dim::Cells(20.0),
            height: Dim::Cells(10.0),
            ..NodeStyle::default()
        };
        let root = rl.add_leaf(style, None).expect("root");
        rl.set_root(root);

        let mut output = Vec::new();
        rl.run_frame(&mut output).expect("frame 1");
        assert!(!rl.needs_render());

        rl.request_full_redraw();
        assert!(rl.needs_render());
        assert!(rl.needs_full_redraw);

        rl.run_frame(&mut output).expect("frame 2 (full redraw)");
        assert!(!rl.needs_render());
    }

    // -- Diff-based output (partial update) --

    #[test]
    fn partial_update_produces_smaller_output() {
        let mut rl = RenderLoop::new(default_config());
        let style = NodeStyle {
            width: Dim::Cells(20.0),
            height: Dim::Cells(10.0),
            ..NodeStyle::default()
        };
        let root = rl
            .add_leaf(
                style,
                Some(Box::new(|layout, buf| {
                    buf.put_str(layout.y as usize, layout.x as usize, "Test", Style::new());
                })),
            )
            .expect("root");
        rl.set_root(root);

        // Frame 1: full redraw (large output)
        let mut output1 = Vec::new();
        let m1 = rl.run_frame(&mut output1).expect("frame 1");

        // Frame 2: mark dirty but same content -> diff should be small or empty
        rl.mark_dirty(root);
        let mut output2 = Vec::new();
        let m2 = rl.run_frame(&mut output2).expect("frame 2");

        // The full render should be at least as large as the diff
        assert!(m1.output_bytes >= m2.output_bytes);
    }

    // -- Error display --

    #[test]
    fn error_display() {
        assert_eq!(format!("{}", RenderLoopError::NoRoot), "no root node set");
        assert_eq!(
            format!("{}", RenderLoopError::LayoutError),
            "layout computation failed"
        );
        assert_eq!(
            format!("{}", RenderLoopError::IoError("broken pipe".into())),
            "IO error: broken pipe"
        );
    }

    // -- Concurrent frame timing --

    #[test]
    fn time_until_next_frame_after_elapsed() {
        let rl = RenderLoop::new(RenderLoopConfig {
            target_fps: 1000, // 1ms interval
            ..default_config()
        });
        // Simulate a frame that happened 100ms ago
        let past = Instant::now() - Duration::from_millis(100);
        let remaining = rl.time_until_next_frame(past);
        assert_eq!(remaining, Duration::ZERO);
    }
}
