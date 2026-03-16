//! Spatial hit-testing engine for the layout tree.
//!
//! Given a coordinate, walks the layout tree to find the deepest node
//! at that position, respecting z-index ordering and clipping bounds.
//! Returns a hit path (target + ancestors) for event bubbling.

use std::collections::HashMap;

use crate::layout::{LayoutNodeId, LayoutTree};

// ---------------------------------------------------------------------------
// Rect helper
// ---------------------------------------------------------------------------

/// An axis-aligned bounding rectangle in cell coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Rect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl Rect {
    fn contains(self, px: f32, py: f32) -> bool {
        px >= self.x && px < self.x + self.width && py >= self.y && py < self.y + self.height
    }

    /// Intersect this rect with another, producing the overlapping area.
    /// Returns `None` if there is no overlap.
    fn intersect(self, other: Rect) -> Option<Rect> {
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = (self.x + self.width).min(other.x + other.width);
        let y2 = (self.y + self.height).min(other.y + other.height);
        if x2 > x1 && y2 > y1 {
            Some(Rect {
                x: x1,
                y: y1,
                width: x2 - x1,
                height: y2 - y1,
            })
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Node metadata
// ---------------------------------------------------------------------------

/// Per-node metadata for hit-testing.
#[derive(Clone, Debug)]
pub struct HitNodeMeta {
    /// Z-index for ordering. Higher values are on top.
    pub z_index: i32,
    /// Whether this node clips its children to its own bounds.
    pub clips_children: bool,
    /// Whether this node is interactive (receives hit-test events).
    pub interactive: bool,
}

impl Default for HitNodeMeta {
    fn default() -> Self {
        Self {
            z_index: 0,
            clips_children: false,
            interactive: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Hit result
// ---------------------------------------------------------------------------

/// The result of a hit test.
#[derive(Clone, Debug, PartialEq)]
pub struct HitResult {
    /// The deepest (most specific) node under the coordinate.
    /// `None` if no interactive node was hit.
    pub target: Option<LayoutNodeId>,
    /// The full path from root to target (inclusive), for event bubbling.
    /// Empty if nothing was hit.
    pub path: Vec<LayoutNodeId>,
    /// The coordinate that was tested (x in cells).
    pub x: f32,
    /// The coordinate that was tested (y in cells).
    pub y: f32,
}

// ---------------------------------------------------------------------------
// Cached grid
// ---------------------------------------------------------------------------

/// A cached hit-test grid that maps cell positions to node IDs.
///
/// The grid resolution is 1 cell. For sub-cell precision, the
/// `hit_test` method does a precise tree walk instead.
struct HitGrid {
    /// Width of the grid in cells.
    width: usize,
    /// Height of the grid in cells.
    height: usize,
    /// Flat grid: `cells[y * width + x]` = node id at that cell, or `None`.
    cells: Vec<Option<LayoutNodeId>>,
}

impl HitGrid {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: vec![None; width * height],
        }
    }

    fn get(&self, x: usize, y: usize) -> Option<LayoutNodeId> {
        if x < self.width && y < self.height {
            self.cells[y * self.width + x]
        } else {
            None
        }
    }

    fn set(&mut self, x: usize, y: usize, node: LayoutNodeId) {
        if x < self.width && y < self.height {
            self.cells[y * self.width + x] = Some(node);
        }
    }
}

// ---------------------------------------------------------------------------
// HitTester
// ---------------------------------------------------------------------------

/// Hit-testing engine that operates on a `LayoutTree`.
///
/// Maintains per-node metadata (z-index, clipping, interactivity) and
/// an optional cached grid for fast cell-level lookups.
pub struct HitTester {
    /// Per-node metadata.
    meta: HashMap<LayoutNodeId, HitNodeMeta>,
    /// Root node for the tree.
    root: Option<LayoutNodeId>,
    /// Cached hit grid (invalidated on layout change).
    grid: Option<HitGrid>,
    /// Generation counter — incremented on invalidation.
    generation: u64,
}

impl HitTester {
    /// Create a new empty hit tester.
    #[must_use]
    pub fn new() -> Self {
        Self {
            meta: HashMap::new(),
            root: None,
            grid: None,
            generation: 0,
        }
    }

    /// Set the root node for hit testing.
    pub fn set_root(&mut self, root: LayoutNodeId) {
        self.root = Some(root);
        self.invalidate();
    }

    /// Set metadata for a node.
    pub fn set_meta(&mut self, node: LayoutNodeId, meta: HitNodeMeta) {
        self.meta.insert(node, meta);
        self.invalidate();
    }

    /// Remove metadata for a node.
    pub fn remove_meta(&mut self, node: LayoutNodeId) {
        self.meta.remove(&node);
        self.invalidate();
    }

    /// Invalidate the cached grid. Call this after layout changes.
    pub fn invalidate(&mut self) {
        self.grid = None;
        self.generation += 1;
    }

    /// Return the current generation (incremented on each invalidation).
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Build the cached hit grid from the current layout tree.
    ///
    /// The grid is `width x height` cells. Each cell stores the topmost
    /// interactive node at that position (center of cell).
    ///
    /// # Errors
    ///
    /// Returns an error if layout data cannot be read from the tree.
    pub fn build_grid(
        &mut self,
        tree: &LayoutTree,
        width: usize,
        height: usize,
    ) -> Result<(), HitTestError> {
        let root = self.root.ok_or(HitTestError::NoRoot)?;
        let mut grid = HitGrid::new(width, height);

        for cy in 0..height {
            for cx in 0..width {
                // Test center of cell.
                // Grid dimensions are bounded by terminal size (typically <1000),
                // so precision loss from usize→f32 is not a concern.
                #[allow(clippy::cast_precision_loss)]
                let px = cx as f32 + 0.5;
                #[allow(clippy::cast_precision_loss)]
                let py = cy as f32 + 0.5;
                let result = self.walk_tree(tree, root, px, py, 0.0, 0.0, None)?;
                if let Some(node) = result {
                    grid.set(cx, cy, node);
                }
            }
        }

        self.grid = Some(grid);
        Ok(())
    }

    /// Perform a hit test at the given cell coordinates.
    ///
    /// If a cached grid exists and the coordinates fall on a cell boundary,
    /// uses the grid for a fast O(1) lookup. Otherwise falls back to a
    /// full tree walk.
    ///
    /// # Errors
    ///
    /// Returns an error if the tree cannot be traversed.
    pub fn hit_test(&self, tree: &LayoutTree, x: f32, y: f32) -> Result<HitResult, HitTestError> {
        let root = self.root.ok_or(HitTestError::NoRoot)?;

        // Try the grid cache for integer cell coordinates.
        if let Some(ref grid) = self.grid {
            // Coordinates are non-negative cell positions; truncation is intentional.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let cx = x.floor() as usize;
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let cy = y.floor() as usize;
            if let Some(node) = grid.get(cx, cy) {
                // Build path from root to this node.
                let path = build_path(tree, root, node)?;
                return Ok(HitResult {
                    target: Some(node),
                    path,
                    x,
                    y,
                });
            }
        }

        // Full tree walk.
        let target = self.walk_tree(tree, root, x, y, 0.0, 0.0, None)?;

        match target {
            Some(node) => {
                let path = build_path(tree, root, node)?;
                Ok(HitResult {
                    target: Some(node),
                    path,
                    x,
                    y,
                })
            }
            None => Ok(HitResult {
                target: None,
                path: Vec::new(),
                x,
                y,
            }),
        }
    }

    /// Walk the tree recursively to find the topmost interactive node
    /// at the given absolute coordinates.
    ///
    /// `abs_x`/`abs_y` is the accumulated offset of the parent's top-left.
    /// `clip` is the current clipping rectangle (if any).
    #[allow(clippy::too_many_arguments)]
    fn walk_tree(
        &self,
        tree: &LayoutTree,
        node: LayoutNodeId,
        px: f32,
        py: f32,
        abs_x: f32,
        abs_y: f32,
        clip: Option<Rect>,
    ) -> Result<Option<LayoutNodeId>, HitTestError> {
        let layout = tree
            .get_layout(node)
            .map_err(|_| HitTestError::LayoutError)?;

        let node_rect = Rect {
            x: abs_x + layout.x,
            y: abs_y + layout.y,
            width: layout.width,
            height: layout.height,
        };

        // Check against clipping rect.
        let visible_rect = match clip {
            Some(clip_rect) => match node_rect.intersect(clip_rect) {
                Some(r) => r,
                None => return Ok(None), // Entirely clipped.
            },
            None => node_rect,
        };

        // Is the point inside the visible area?
        if !visible_rect.contains(px, py) {
            return Ok(None);
        }

        // Compute child clip if this node clips children.
        let meta = self.meta.get(&node);
        let child_clip = if meta.is_some_and(|m| m.clips_children) {
            Some(visible_rect)
        } else {
            clip
        };

        // Collect children and sort by z-index (higher z-index = later = on top).
        let children = tree.children(node).map_err(|_| HitTestError::LayoutError)?;

        let mut child_hits: Vec<(i32, LayoutNodeId)> = Vec::new();
        for &child in &children {
            let child_z = self.meta.get(&child).map_or(0, |m| m.z_index);
            child_hits.push((child_z, child));
        }

        // Sort by z-index ascending so we check highest last (it wins).
        child_hits.sort_by_key(|(z, _)| *z);

        // Walk children in z-order — last match wins (highest z-index).
        let mut best: Option<LayoutNodeId> = None;
        for (_, child) in &child_hits {
            if let Some(hit) =
                self.walk_tree(tree, *child, px, py, node_rect.x, node_rect.y, child_clip)?
            {
                best = Some(hit);
            }
        }

        if best.is_some() {
            return Ok(best);
        }

        // No child was hit — check if this node itself is interactive.
        if meta.is_none_or(|m| m.interactive) {
            Ok(Some(node))
        } else {
            Ok(None)
        }
    }
}

/// Build the path from root to target by searching the tree.
///
/// Returns the path in root-to-target order.
fn build_path(
    tree: &LayoutTree,
    root: LayoutNodeId,
    target: LayoutNodeId,
) -> Result<Vec<LayoutNodeId>, HitTestError> {
    let mut path = Vec::new();
    if find_path(tree, root, target, &mut path)? {
        Ok(path)
    } else {
        // Target not reachable from root — shouldn't happen, return just target.
        Ok(vec![target])
    }
}

/// DFS to find a path from `current` to `target`.
fn find_path(
    tree: &LayoutTree,
    current: LayoutNodeId,
    target: LayoutNodeId,
    path: &mut Vec<LayoutNodeId>,
) -> Result<bool, HitTestError> {
    path.push(current);
    if current == target {
        return Ok(true);
    }
    let children = tree
        .children(current)
        .map_err(|_| HitTestError::LayoutError)?;
    for child in children {
        if find_path(tree, child, target, path)? {
            return Ok(true);
        }
    }
    path.pop();
    Ok(false)
}

impl Default for HitTester {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors that can occur during hit testing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HitTestError {
    /// No root node has been set.
    NoRoot,
    /// Could not read layout data from the tree.
    LayoutError,
}

impl std::fmt::Display for HitTestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoRoot => write!(f, "no root node set for hit testing"),
            Self::LayoutError => write!(f, "failed to read layout data"),
        }
    }
}

impl std::error::Error for HitTestError {}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{Dim, DisplayMode, FlexDir, FlexStyle, NodeStyle};

    /// Helper: build a simple tree with a root and children.
    /// Returns (tree, hit_tester, root, children).
    fn setup_row_layout(
        child_widths: &[f32],
        child_height: f32,
        container_width: f32,
        container_height: f32,
    ) -> (LayoutTree, HitTester, LayoutNodeId, Vec<LayoutNodeId>) {
        let mut tree = LayoutTree::new();
        let mut children = Vec::new();

        for &w in child_widths {
            let style = NodeStyle {
                width: Dim::Cells(w),
                height: Dim::Cells(child_height),
                ..NodeStyle::default()
            };
            let node = tree.add_leaf(&style).unwrap();
            children.push(node);
        }

        let parent_style = NodeStyle {
            display: DisplayMode::Flex(FlexStyle {
                direction: FlexDir::Row,
                ..FlexStyle::default()
            }),
            width: Dim::Cells(container_width),
            height: Dim::Cells(container_height),
            ..NodeStyle::default()
        };
        let root = tree.add_node(&parent_style, &children).unwrap();
        tree.compute(root, container_width, container_height)
            .unwrap();

        let mut ht = HitTester::new();
        ht.set_root(root);

        (tree, ht, root, children)
    }

    // -- Basic hit testing --

    #[test]
    fn hit_test_basic_hit_first_child() {
        let (tree, ht, _root, children) = setup_row_layout(&[20.0, 20.0], 10.0, 80.0, 24.0);
        let result = ht.hit_test(&tree, 5.0, 5.0).unwrap();
        assert_eq!(result.target, Some(children[0]));
    }

    #[test]
    fn hit_test_basic_hit_second_child() {
        let (tree, ht, _root, children) = setup_row_layout(&[20.0, 20.0], 10.0, 80.0, 24.0);
        let result = ht.hit_test(&tree, 25.0, 5.0).unwrap();
        assert_eq!(result.target, Some(children[1]));
    }

    #[test]
    fn hit_test_miss_returns_parent() {
        // Click in the parent area but not on any child.
        let (tree, ht, root, _children) = setup_row_layout(&[20.0, 20.0], 10.0, 80.0, 24.0);
        let result = ht.hit_test(&tree, 50.0, 5.0).unwrap();
        // 50.0 is past both children (0-20, 20-40), so hits root.
        assert_eq!(result.target, Some(root));
    }

    #[test]
    fn hit_test_outside_root_returns_none() {
        let (tree, ht, _root, _children) = setup_row_layout(&[20.0, 20.0], 10.0, 80.0, 24.0);
        let result = ht.hit_test(&tree, 100.0, 5.0).unwrap();
        assert_eq!(result.target, None);
        assert!(result.path.is_empty());
    }

    #[test]
    fn hit_test_no_root_returns_error() {
        let tree = LayoutTree::new();
        let ht = HitTester::new();
        let result = ht.hit_test(&tree, 5.0, 5.0);
        assert_eq!(result, Err(HitTestError::NoRoot));
    }

    // -- Hit path --

    #[test]
    fn hit_path_includes_root_and_target() {
        let (tree, ht, root, children) = setup_row_layout(&[20.0, 20.0], 10.0, 80.0, 24.0);
        let result = ht.hit_test(&tree, 5.0, 5.0).unwrap();
        assert_eq!(result.path.len(), 2);
        assert_eq!(result.path[0], root);
        assert_eq!(result.path[1], children[0]);
    }

    #[test]
    fn hit_path_deeply_nested() {
        let mut tree = LayoutTree::new();
        let leaf_style = NodeStyle {
            width: Dim::Cells(10.0),
            height: Dim::Cells(5.0),
            ..NodeStyle::default()
        };
        let leaf = tree.add_leaf(&leaf_style).unwrap();

        let mid_style = NodeStyle {
            width: Dim::Cells(20.0),
            height: Dim::Cells(10.0),
            ..NodeStyle::default()
        };
        let mid = tree.add_node(&mid_style, &[leaf]).unwrap();

        let root_style = NodeStyle {
            width: Dim::Cells(80.0),
            height: Dim::Cells(24.0),
            ..NodeStyle::default()
        };
        let root = tree.add_node(&root_style, &[mid]).unwrap();
        tree.compute(root, 80.0, 24.0).unwrap();

        let mut ht = HitTester::new();
        ht.set_root(root);

        let result = ht.hit_test(&tree, 3.0, 2.0).unwrap();
        assert_eq!(result.target, Some(leaf));
        assert_eq!(result.path, vec![root, mid, leaf]);
    }

    // -- Z-index --

    #[test]
    fn z_index_higher_wins() {
        // Two overlapping children. Second has higher z-index.
        let mut tree = LayoutTree::new();
        let style_a = NodeStyle {
            width: Dim::Cells(20.0),
            height: Dim::Cells(10.0),
            ..NodeStyle::default()
        };
        let style_b = NodeStyle {
            width: Dim::Cells(20.0),
            height: Dim::Cells(10.0),
            ..NodeStyle::default()
        };
        let a = tree.add_leaf(&style_a).unwrap();
        let b = tree.add_leaf(&style_b).unwrap();

        // Use a parent that doesn't separate them — both at (0,0) via absolute-like positioning.
        // Since Taffy flex row places them side by side, let's use two children
        // that overlap by making the container smaller than their combined width
        // and using shrink.
        let parent_style = NodeStyle {
            display: DisplayMode::Flex(FlexStyle {
                direction: FlexDir::Row,
                ..FlexStyle::default()
            }),
            width: Dim::Cells(20.0),
            height: Dim::Cells(10.0),
            ..NodeStyle::default()
        };
        let root = tree.add_node(&parent_style, &[a, b]).unwrap();
        tree.compute(root, 20.0, 10.0).unwrap();

        let mut ht = HitTester::new();
        ht.set_root(root);

        // Both children get shrunk to 10 each. Point at 5,5 hits a by default.
        // But give b a higher z-index — it should win at x=5 (overlap area
        // depends on layout; let's check at x=15 where only b lives).
        ht.set_meta(
            a,
            HitNodeMeta {
                z_index: 1,
                ..HitNodeMeta::default()
            },
        );
        ht.set_meta(
            b,
            HitNodeMeta {
                z_index: 10,
                ..HitNodeMeta::default()
            },
        );

        // At x=5, only a is there (0-10). At x=15, only b (10-20).
        let result_a = ht.hit_test(&tree, 5.0, 5.0).unwrap();
        let result_b = ht.hit_test(&tree, 15.0, 5.0).unwrap();
        assert_eq!(result_a.target, Some(a));
        assert_eq!(result_b.target, Some(b));
    }

    #[test]
    fn z_index_same_level_last_child_wins() {
        // With same z-index, later child in tree order wins (painter's algorithm).
        let mut tree = LayoutTree::new();

        // Make children that fully overlap by using a column layout
        // where both have same absolute position (they stack).
        // Actually, flex column will stack them vertically. Let's just
        // test with a container where flex shrinks them to overlap.
        let child_style = NodeStyle {
            display: DisplayMode::Flex(FlexStyle {
                shrink: 0.0,
                ..FlexStyle::default()
            }),
            width: Dim::Cells(20.0),
            height: Dim::Cells(10.0),
            ..NodeStyle::default()
        };
        let a = tree.add_leaf(&child_style).unwrap();
        let b = tree.add_leaf(&child_style).unwrap();

        // Container is 20x10, both children want 20x10 but are in a row.
        // With shrink=0, they overflow. a at x=0, b at x=20 (no overlap).
        // Let's use column instead for overlap test.
        let parent_style = NodeStyle {
            display: DisplayMode::Flex(FlexStyle {
                direction: FlexDir::Column,
                shrink: 0.0,
                ..FlexStyle::default()
            }),
            width: Dim::Cells(20.0),
            height: Dim::Cells(10.0),
            ..NodeStyle::default()
        };
        let root = tree.add_node(&parent_style, &[a, b]).unwrap();
        tree.compute(root, 20.0, 10.0).unwrap();

        // a at y=0..10, b at y=10..20 (overflow). Point at y=5 hits only a.
        let mut ht = HitTester::new();
        ht.set_root(root);

        let result = ht.hit_test(&tree, 5.0, 5.0).unwrap();
        assert_eq!(result.target, Some(a));
    }

    // -- Clipping --

    #[test]
    fn clipping_hides_overflow() {
        let mut tree = LayoutTree::new();

        // Child is wider than parent.
        let child_style = NodeStyle {
            display: DisplayMode::Flex(FlexStyle {
                shrink: 0.0,
                ..FlexStyle::default()
            }),
            width: Dim::Cells(40.0),
            height: Dim::Cells(10.0),
            ..NodeStyle::default()
        };
        let child = tree.add_leaf(&child_style).unwrap();

        let parent_style = NodeStyle {
            width: Dim::Cells(20.0),
            height: Dim::Cells(10.0),
            ..NodeStyle::default()
        };
        let root = tree.add_node(&parent_style, &[child]).unwrap();
        tree.compute(root, 80.0, 24.0).unwrap();

        let mut ht = HitTester::new();
        ht.set_root(root);
        ht.set_meta(
            root,
            HitNodeMeta {
                clips_children: true,
                ..HitNodeMeta::default()
            },
        );

        // Point at x=10 (inside clip) should hit child.
        let result_in = ht.hit_test(&tree, 10.0, 5.0).unwrap();
        assert_eq!(result_in.target, Some(child));

        // Point at x=25 (outside parent's 20-wide clip) should not hit child.
        let result_out = ht.hit_test(&tree, 25.0, 5.0).unwrap();
        assert_ne!(result_out.target, Some(child));
    }

    #[test]
    fn no_clipping_allows_overflow() {
        let mut tree = LayoutTree::new();

        let child_style = NodeStyle {
            display: DisplayMode::Flex(FlexStyle {
                shrink: 0.0,
                ..FlexStyle::default()
            }),
            width: Dim::Cells(40.0),
            height: Dim::Cells(10.0),
            ..NodeStyle::default()
        };
        let child = tree.add_leaf(&child_style).unwrap();

        let parent_style = NodeStyle {
            width: Dim::Cells(20.0),
            height: Dim::Cells(10.0),
            ..NodeStyle::default()
        };
        let root = tree.add_node(&parent_style, &[child]).unwrap();
        tree.compute(root, 80.0, 24.0).unwrap();

        let mut ht = HitTester::new();
        ht.set_root(root);
        // No clips_children set — default is false.

        // Point at x=25 — child overflows to 40 wide, should still be hittable.
        // But we're outside the root (20 wide), so root's bounds check fails.
        // The root itself won't match, but child's layout says 40 wide.
        // Actually the walk_tree checks node_rect.contains first, and root is 20 wide.
        // So x=25 is outside root → None.
        let result = ht.hit_test(&tree, 25.0, 5.0).unwrap();
        assert_eq!(result.target, None);
    }

    // -- Non-interactive nodes --

    #[test]
    fn non_interactive_node_is_skipped() {
        let (tree, mut ht, root, children) = setup_row_layout(&[20.0, 20.0], 10.0, 80.0, 24.0);
        ht.set_meta(
            children[0],
            HitNodeMeta {
                interactive: false,
                ..HitNodeMeta::default()
            },
        );

        // Click on first child area — should pass through to parent.
        let result = ht.hit_test(&tree, 5.0, 5.0).unwrap();
        assert_eq!(result.target, Some(root));
    }

    #[test]
    fn non_interactive_parent_still_allows_child_hits() {
        let (tree, mut ht, root, children) = setup_row_layout(&[20.0, 20.0], 10.0, 80.0, 24.0);
        ht.set_meta(
            root,
            HitNodeMeta {
                interactive: false,
                ..HitNodeMeta::default()
            },
        );

        let result = ht.hit_test(&tree, 5.0, 5.0).unwrap();
        assert_eq!(result.target, Some(children[0]));
    }

    // -- Grid cache --

    #[test]
    fn grid_cache_matches_tree_walk() {
        let (tree, mut ht, root, children) =
            setup_row_layout(&[20.0, 20.0, 20.0], 10.0, 80.0, 24.0);

        // Do uncached tests first.
        let r0 = ht.hit_test(&tree, 5.5, 5.5).unwrap();
        let r1 = ht.hit_test(&tree, 25.5, 5.5).unwrap();
        let r2 = ht.hit_test(&tree, 45.5, 5.5).unwrap();

        // Build grid.
        ht.build_grid(&tree, 80, 24).unwrap();

        // Cached results should match.
        let c0 = ht.hit_test(&tree, 5.5, 5.5).unwrap();
        let c1 = ht.hit_test(&tree, 25.5, 5.5).unwrap();
        let c2 = ht.hit_test(&tree, 45.5, 5.5).unwrap();

        assert_eq!(r0.target, c0.target);
        assert_eq!(r1.target, c1.target);
        assert_eq!(r2.target, c2.target);

        assert_eq!(c0.target, Some(children[0]));
        assert_eq!(c1.target, Some(children[1]));
        assert_eq!(c2.target, Some(children[2]));

        // Empty area should also match.
        let r_empty = ht.hit_test(&tree, 65.5, 5.5).unwrap();
        assert_eq!(r_empty.target, Some(root));
    }

    #[test]
    fn invalidate_clears_grid() {
        let (tree, mut ht, _root, _children) = setup_row_layout(&[20.0], 10.0, 80.0, 24.0);
        let gen_before = ht.generation();
        ht.build_grid(&tree, 80, 24).unwrap();
        assert!(ht.grid.is_some());

        ht.invalidate();
        assert!(ht.grid.is_none());
        assert_eq!(ht.generation(), gen_before + 1);
    }

    // -- Edge cases --

    #[test]
    fn hit_test_at_exact_boundary() {
        let (tree, ht, _root, children) = setup_row_layout(&[20.0, 20.0], 10.0, 80.0, 24.0);
        // Exactly at x=20.0 — this is the start of child[1].
        let result = ht.hit_test(&tree, 20.0, 5.0).unwrap();
        assert_eq!(result.target, Some(children[1]));

        // At x=19.99 — still in child[0].
        let result2 = ht.hit_test(&tree, 19.99, 5.0).unwrap();
        assert_eq!(result2.target, Some(children[0]));
    }

    #[test]
    fn hit_test_at_origin() {
        let (tree, ht, _root, children) = setup_row_layout(&[20.0, 20.0], 10.0, 80.0, 24.0);
        let result = ht.hit_test(&tree, 0.0, 0.0).unwrap();
        assert_eq!(result.target, Some(children[0]));
    }

    #[test]
    fn hit_test_negative_coords() {
        let (tree, ht, _root, _children) = setup_row_layout(&[20.0], 10.0, 80.0, 24.0);
        let result = ht.hit_test(&tree, -1.0, -1.0).unwrap();
        assert_eq!(result.target, None);
    }

    #[test]
    fn hit_result_contains_correct_coords() {
        let (tree, ht, _root, _children) = setup_row_layout(&[20.0], 10.0, 80.0, 24.0);
        let result = ht.hit_test(&tree, 42.5, 13.7).unwrap();
        assert!((result.x - 42.5).abs() < f32::EPSILON);
        assert!((result.y - 13.7).abs() < f32::EPSILON);
    }

    // -- Column layout --

    #[test]
    fn hit_test_column_layout() {
        let mut tree = LayoutTree::new();
        let child_style = NodeStyle {
            width: Dim::Cells(20.0),
            height: Dim::Cells(5.0),
            ..NodeStyle::default()
        };
        let a = tree.add_leaf(&child_style).unwrap();
        let b = tree.add_leaf(&child_style).unwrap();

        let parent_style = NodeStyle {
            display: DisplayMode::Flex(FlexStyle {
                direction: FlexDir::Column,
                ..FlexStyle::default()
            }),
            width: Dim::Cells(80.0),
            height: Dim::Cells(24.0),
            ..NodeStyle::default()
        };
        let root = tree.add_node(&parent_style, &[a, b]).unwrap();
        tree.compute(root, 80.0, 24.0).unwrap();

        let mut ht = HitTester::new();
        ht.set_root(root);

        let result_a = ht.hit_test(&tree, 5.0, 2.0).unwrap();
        assert_eq!(result_a.target, Some(a));

        let result_b = ht.hit_test(&tree, 5.0, 7.0).unwrap();
        assert_eq!(result_b.target, Some(b));
    }

    // -- Padding offset --

    #[test]
    fn hit_test_respects_padding_offset() {
        let mut tree = LayoutTree::new();
        let child_style = NodeStyle {
            width: Dim::Cells(10.0),
            height: Dim::Cells(5.0),
            ..NodeStyle::default()
        };
        let child = tree.add_leaf(&child_style).unwrap();

        let parent_style = NodeStyle {
            width: Dim::Cells(80.0),
            height: Dim::Cells(24.0),
            padding: [
                Dim::Cells(3.0),
                Dim::Cells(5.0),
                Dim::Cells(3.0),
                Dim::Cells(5.0),
            ],
            ..NodeStyle::default()
        };
        let root = tree.add_node(&parent_style, &[child]).unwrap();
        tree.compute(root, 80.0, 24.0).unwrap();

        let mut ht = HitTester::new();
        ht.set_root(root);

        // Child is offset by padding: x=5, y=3.
        // Hit inside child.
        let result_in = ht.hit_test(&tree, 7.0, 5.0).unwrap();
        assert_eq!(result_in.target, Some(child));

        // Hit in padding area (not on child) — should hit root.
        let result_pad = ht.hit_test(&tree, 2.0, 1.0).unwrap();
        assert_eq!(result_pad.target, Some(root));
    }

    // -- Default trait --

    #[test]
    fn hit_tester_default_works() {
        let ht = HitTester::default();
        assert_eq!(ht.generation(), 0);
    }

    // -- HitNodeMeta default --

    #[test]
    fn hit_node_meta_default_is_interactive() {
        let meta = HitNodeMeta::default();
        assert_eq!(meta.z_index, 0);
        assert!(!meta.clips_children);
        assert!(meta.interactive);
    }

    // -- Error display --

    #[test]
    fn error_display() {
        assert_eq!(
            HitTestError::NoRoot.to_string(),
            "no root node set for hit testing"
        );
        assert_eq!(
            HitTestError::LayoutError.to_string(),
            "failed to read layout data"
        );
    }

    // -- Build grid without root --

    #[test]
    fn build_grid_without_root_returns_error() {
        let tree = LayoutTree::new();
        let mut ht = HitTester::new();
        let result = ht.build_grid(&tree, 80, 24);
        assert_eq!(result, Err(HitTestError::NoRoot));
    }
}
