//! Layout engine integration using Taffy for flexbox and grid support.
//!
//! This module wraps Taffy's `TaffyTree` to provide a layout system that
//! works in terminal cell coordinates. All dimensions are ultimately
//! expressed in cells (columns x rows), but callers can specify sizes
//! as cells, percentages, or auto.

use taffy::prelude::*;
use taffy::TaffyTree;

// Re-export taffy alignment types so downstream modules (e.g. ffi_bridge) do
// not need a direct `taffy::` dependency which can cause linker/code-signing
// issues on macOS when the symbol set changes between incremental builds.
pub use taffy::{AlignItems, JustifyContent};

// ---------------------------------------------------------------------------
// Dimension helpers
// ---------------------------------------------------------------------------

/// A dimension value in the `KittyUI` coordinate system.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Dim {
    /// Fixed size in terminal cells.
    Cells(f32),
    /// Percentage of the parent container (0.0–100.0).
    Percent(f32),
    /// Size determined automatically by content / layout algorithm.
    Auto,
}

impl Dim {
    /// Convert to a Taffy [`Dimension`].
    #[must_use]
    fn to_taffy(self) -> Dimension {
        match self {
            Self::Cells(v) => Dimension::Length(v),
            Self::Percent(v) => Dimension::Percent(v / 100.0),
            Self::Auto => Dimension::Auto,
        }
    }

    /// Convert to a Taffy [`LengthPercentage`] (no auto variant).
    #[must_use]
    fn to_taffy_lp(self) -> LengthPercentage {
        match self {
            Self::Cells(v) => LengthPercentage::Length(v),
            Self::Percent(v) => LengthPercentage::Percent(v / 100.0),
            Self::Auto => LengthPercentage::Length(0.0),
        }
    }

    /// Convert to a Taffy [`LengthPercentageAuto`].
    #[must_use]
    fn to_taffy_lpa(self) -> LengthPercentageAuto {
        match self {
            Self::Cells(v) => LengthPercentageAuto::Length(v),
            Self::Percent(v) => LengthPercentageAuto::Percent(v / 100.0),
            Self::Auto => LengthPercentageAuto::Auto,
        }
    }
}

// ---------------------------------------------------------------------------
// Cell dimensions for pixel↔cell conversion
// ---------------------------------------------------------------------------

/// Terminal cell dimensions in pixels.
///
/// Used to convert pixel-based values to cell coordinates when needed.
#[derive(Clone, Copy, Debug)]
pub struct CellSize {
    /// Width of one cell in pixels.
    pub width_px: f32,
    /// Height of one cell in pixels.
    pub height_px: f32,
}

impl CellSize {
    /// Create a new `CellSize`.
    #[must_use]
    pub fn new(width_px: f32, height_px: f32) -> Self {
        Self {
            width_px,
            height_px,
        }
    }

    /// Convert pixel width to cell columns.
    #[must_use]
    pub fn px_to_cols(&self, px: f32) -> f32 {
        if self.width_px > 0.0 {
            px / self.width_px
        } else {
            0.0
        }
    }

    /// Convert pixel height to cell rows.
    #[must_use]
    pub fn px_to_rows(&self, px: f32) -> f32 {
        if self.height_px > 0.0 {
            px / self.height_px
        } else {
            0.0
        }
    }

    /// Convert cell columns to pixels.
    #[must_use]
    pub fn cols_to_px(&self, cols: f32) -> f32 {
        cols * self.width_px
    }

    /// Convert cell rows to pixels.
    #[must_use]
    pub fn rows_to_px(&self, rows: f32) -> f32 {
        rows * self.height_px
    }
}

// ---------------------------------------------------------------------------
// Flexbox style builder
// ---------------------------------------------------------------------------

/// Flexbox-specific style properties.
#[derive(Clone, Debug)]
pub struct FlexStyle {
    /// Main axis direction.
    pub direction: FlexDir,
    /// Wrap behaviour.
    pub wrap: Wrap,
    /// Justify content along the main axis.
    pub justify: JustifyContent,
    /// Align items along the cross axis.
    pub align_items: AlignItems,
    /// Flex grow factor.
    pub grow: f32,
    /// Flex shrink factor.
    pub shrink: f32,
    /// Flex basis.
    pub basis: Dim,
}

impl Default for FlexStyle {
    fn default() -> Self {
        Self {
            direction: FlexDir::Row,
            wrap: Wrap::NoWrap,
            justify: JustifyContent::Start,
            align_items: AlignItems::Stretch,
            grow: 0.0,
            shrink: 1.0,
            basis: Dim::Auto,
        }
    }
}

/// Flex direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlexDir {
    Row,
    Column,
    RowReverse,
    ColumnReverse,
}

/// Flex wrap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wrap {
    NoWrap,
    Wrap,
    WrapReverse,
}

// ---------------------------------------------------------------------------
// Grid style builder
// ---------------------------------------------------------------------------

/// Grid-specific style properties.
#[derive(Clone, Debug)]
pub struct GridStyle {
    /// Column track definitions.
    pub columns: Vec<TrackDef>,
    /// Row track definitions.
    pub rows: Vec<TrackDef>,
    /// Gap between columns.
    pub column_gap: Dim,
    /// Gap between rows.
    pub row_gap: Dim,
}

impl Default for GridStyle {
    fn default() -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
            column_gap: Dim::Cells(0.0),
            row_gap: Dim::Cells(0.0),
        }
    }
}

/// A single grid track definition.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TrackDef {
    /// Fixed size in cells.
    Cells(f32),
    /// Fractional unit (like CSS `fr`).
    Fr(f32),
    /// Percentage of the container.
    Percent(f32),
    /// Auto-sized.
    Auto,
}

impl TrackDef {
    fn to_taffy(self) -> TrackSizingFunction {
        match self {
            Self::Cells(v) => length(v),
            Self::Fr(v) => fr(v),
            Self::Percent(v) => percent(v / 100.0),
            Self::Auto => auto(),
        }
    }
}

// ---------------------------------------------------------------------------
// Node style
// ---------------------------------------------------------------------------

/// Display mode for a layout node.
#[derive(Clone, Debug)]
pub enum DisplayMode {
    Flex(FlexStyle),
    Grid(GridStyle),
}

/// Complete style for a layout node.
#[derive(Clone, Debug)]
pub struct NodeStyle {
    /// Display mode (flex or grid).
    pub display: DisplayMode,
    /// Width.
    pub width: Dim,
    /// Height.
    pub height: Dim,
    /// Min width.
    pub min_width: Dim,
    /// Min height.
    pub min_height: Dim,
    /// Max width.
    pub max_width: Dim,
    /// Max height.
    pub max_height: Dim,
    /// Padding (top, right, bottom, left) in cells.
    pub padding: [Dim; 4],
    /// Margin (top, right, bottom, left).
    pub margin: [Dim; 4],
    /// Gap for flex layouts.
    pub gap: [Dim; 2],
}

impl Default for NodeStyle {
    fn default() -> Self {
        Self {
            display: DisplayMode::Flex(FlexStyle::default()),
            width: Dim::Auto,
            height: Dim::Auto,
            min_width: Dim::Auto,
            min_height: Dim::Auto,
            max_width: Dim::Auto,
            max_height: Dim::Auto,
            padding: [Dim::Cells(0.0); 4],
            margin: [Dim::Cells(0.0); 4],
            gap: [Dim::Cells(0.0); 2],
        }
    }
}

impl NodeStyle {
    /// Convert to a Taffy `Style`.
    fn to_taffy(&self) -> Style {
        let mut style = Style {
            size: Size {
                width: self.width.to_taffy(),
                height: self.height.to_taffy(),
            },
            min_size: Size {
                width: self.min_width.to_taffy(),
                height: self.min_height.to_taffy(),
            },
            max_size: Size {
                width: self.max_width.to_taffy(),
                height: self.max_height.to_taffy(),
            },
            padding: Rect {
                top: self.padding[0].to_taffy_lp(),
                right: self.padding[1].to_taffy_lp(),
                bottom: self.padding[2].to_taffy_lp(),
                left: self.padding[3].to_taffy_lp(),
            },
            margin: Rect {
                top: self.margin[0].to_taffy_lpa(),
                right: self.margin[1].to_taffy_lpa(),
                bottom: self.margin[2].to_taffy_lpa(),
                left: self.margin[3].to_taffy_lpa(),
            },
            ..Style::DEFAULT
        };

        match &self.display {
            DisplayMode::Flex(flex) => {
                style.display = Display::Flex;
                style.flex_direction = match flex.direction {
                    FlexDir::Row => FlexDirection::Row,
                    FlexDir::Column => FlexDirection::Column,
                    FlexDir::RowReverse => FlexDirection::RowReverse,
                    FlexDir::ColumnReverse => FlexDirection::ColumnReverse,
                };
                style.flex_wrap = match flex.wrap {
                    Wrap::NoWrap => FlexWrap::NoWrap,
                    Wrap::Wrap => FlexWrap::Wrap,
                    Wrap::WrapReverse => FlexWrap::WrapReverse,
                };
                style.justify_content = Some(flex.justify);
                style.align_items = Some(flex.align_items);
                style.flex_grow = flex.grow;
                style.flex_shrink = flex.shrink;
                style.flex_basis = flex.basis.to_taffy();
                style.gap = Size {
                    width: self.gap[0].to_taffy_lp(),
                    height: self.gap[1].to_taffy_lp(),
                };
            }
            DisplayMode::Grid(grid) => {
                style.display = Display::Grid;
                style.grid_template_columns = grid.columns.iter().map(|t| t.to_taffy()).collect();
                style.grid_template_rows = grid.rows.iter().map(|t| t.to_taffy()).collect();
                style.gap = Size {
                    width: grid.column_gap.to_taffy_lp(),
                    height: grid.row_gap.to_taffy_lp(),
                };
            }
        }

        style
    }
}

// ---------------------------------------------------------------------------
// Computed layout result
// ---------------------------------------------------------------------------

/// Computed layout for a single node, in cell coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ComputedLayout {
    /// X position relative to the parent (columns).
    pub x: f32,
    /// Y position relative to the parent (rows).
    pub y: f32,
    /// Width in columns.
    pub width: f32,
    /// Height in rows.
    pub height: f32,
}

// ---------------------------------------------------------------------------
// LayoutTree
// ---------------------------------------------------------------------------

/// A handle to a node in the layout tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LayoutNodeId(NodeId);

/// Layout tree backed by Taffy.
///
/// Maps `KittyUI`'s component tree to Taffy nodes and exposes computed
/// layout results in cell coordinates.
pub struct LayoutTree {
    taffy: TaffyTree<()>,
}

impl LayoutTree {
    /// Create an empty layout tree.
    #[must_use]
    pub fn new() -> Self {
        Self {
            taffy: TaffyTree::new(),
        }
    }

    /// Add a leaf node (no children) with the given style.
    ///
    /// # Errors
    ///
    /// Returns a `TaffyError` if the node cannot be created.
    pub fn add_leaf(&mut self, style: &NodeStyle) -> Result<LayoutNodeId, taffy::TaffyError> {
        let id = self.taffy.new_leaf(style.to_taffy())?;
        Ok(LayoutNodeId(id))
    }

    /// Add a container node with children.
    ///
    /// # Errors
    ///
    /// Returns a `TaffyError` if the node cannot be created.
    pub fn add_node(
        &mut self,
        style: &NodeStyle,
        children: &[LayoutNodeId],
    ) -> Result<LayoutNodeId, taffy::TaffyError> {
        let child_ids: Vec<NodeId> = children.iter().map(|c| c.0).collect();
        let id = self.taffy.new_with_children(style.to_taffy(), &child_ids)?;
        Ok(LayoutNodeId(id))
    }

    /// Update the style of an existing node.
    ///
    /// # Errors
    ///
    /// Returns a `TaffyError` if the node does not exist.
    pub fn set_style(
        &mut self,
        node: LayoutNodeId,
        style: &NodeStyle,
    ) -> Result<(), taffy::TaffyError> {
        self.taffy.set_style(node.0, style.to_taffy())
    }

    /// Add a child to an existing node.
    ///
    /// # Errors
    ///
    /// Returns a `TaffyError` if either node does not exist.
    pub fn add_child(
        &mut self,
        parent: LayoutNodeId,
        child: LayoutNodeId,
    ) -> Result<(), taffy::TaffyError> {
        self.taffy.add_child(parent.0, child.0)
    }

    /// Remove a node from the tree.
    ///
    /// # Errors
    ///
    /// Returns a `TaffyError` if the node does not exist.
    pub fn remove(&mut self, node: LayoutNodeId) -> Result<(), taffy::TaffyError> {
        self.taffy.remove(node.0).map(|_| ())
    }

    /// Compute the layout of the entire tree, starting from `root`.
    ///
    /// `available_space` is the container size in cells (width, height).
    ///
    /// # Errors
    ///
    /// Returns a `TaffyError` if layout computation fails.
    pub fn compute(
        &mut self,
        root: LayoutNodeId,
        available_width: f32,
        available_height: f32,
    ) -> Result<(), taffy::TaffyError> {
        self.taffy.compute_layout(
            root.0,
            Size {
                width: AvailableSpace::Definite(available_width),
                height: AvailableSpace::Definite(available_height),
            },
        )
    }

    /// Get the computed layout for a node.
    ///
    /// Call [`compute`](Self::compute) first.
    ///
    /// # Errors
    ///
    /// Returns a `TaffyError` if the node does not exist.
    pub fn get_layout(&self, node: LayoutNodeId) -> Result<ComputedLayout, taffy::TaffyError> {
        let layout = self.taffy.layout(node.0)?;
        Ok(ComputedLayout {
            x: layout.location.x,
            y: layout.location.y,
            width: layout.size.width,
            height: layout.size.height,
        })
    }

    /// Return the number of nodes in the tree.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.taffy.total_node_count()
    }

    /// Return the children of a node.
    ///
    /// # Errors
    ///
    /// Returns a `TaffyError` if the node does not exist.
    pub fn children(&self, node: LayoutNodeId) -> Result<Vec<LayoutNodeId>, taffy::TaffyError> {
        let ids = self.taffy.children(node.0)?;
        Ok(ids.into_iter().map(LayoutNodeId).collect())
    }
}

impl Default for LayoutTree {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Dimension conversion tests ---

    #[test]
    fn dim_cells_converts_to_length() {
        let d = Dim::Cells(10.0);
        assert_eq!(d.to_taffy(), Dimension::Length(10.0));
    }

    #[test]
    fn dim_percent_converts_to_fraction() {
        let d = Dim::Percent(50.0);
        assert_eq!(d.to_taffy(), Dimension::Percent(0.5));
    }

    #[test]
    fn dim_auto_converts() {
        assert_eq!(Dim::Auto.to_taffy(), Dimension::Auto);
    }

    // --- CellSize conversion tests ---

    #[test]
    fn cell_size_px_to_cols() {
        let cs = CellSize::new(8.0, 16.0);
        assert!((cs.px_to_cols(80.0) - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn cell_size_px_to_rows() {
        let cs = CellSize::new(8.0, 16.0);
        assert!((cs.px_to_rows(48.0) - 3.0).abs() < f32::EPSILON);
    }

    #[test]
    fn cell_size_roundtrip() {
        let cs = CellSize::new(8.0, 16.0);
        let cols = 25.0;
        let rows = 10.0;
        assert!((cs.px_to_cols(cs.cols_to_px(cols)) - cols).abs() < f32::EPSILON);
        assert!((cs.px_to_rows(cs.rows_to_px(rows)) - rows).abs() < f32::EPSILON);
    }

    #[test]
    fn cell_size_zero_dimensions() {
        let cs = CellSize::new(0.0, 0.0);
        assert!((cs.px_to_cols(100.0)).abs() < f32::EPSILON);
        assert!((cs.px_to_rows(100.0)).abs() < f32::EPSILON);
    }

    // --- LayoutTree basic tests ---

    #[test]
    fn empty_tree() {
        let tree = LayoutTree::new();
        assert_eq!(tree.node_count(), 0);
    }

    #[test]
    fn add_leaf_increments_count() {
        let mut tree = LayoutTree::new();
        let style = NodeStyle::default();
        tree.add_leaf(&style).unwrap();
        assert_eq!(tree.node_count(), 1);
    }

    #[test]
    fn add_node_with_children() {
        let mut tree = LayoutTree::new();
        let leaf_style = NodeStyle {
            width: Dim::Cells(10.0),
            height: Dim::Cells(5.0),
            ..NodeStyle::default()
        };
        let a = tree.add_leaf(&leaf_style).unwrap();
        let b = tree.add_leaf(&leaf_style).unwrap();
        let parent = tree.add_node(&NodeStyle::default(), &[a, b]).unwrap();
        assert_eq!(tree.node_count(), 3);
        // Parent should exist and be queryable after compute.
        tree.compute(parent, 80.0, 24.0).unwrap();
        let layout = tree.get_layout(parent).unwrap();
        assert!(layout.width > 0.0);
    }

    #[test]
    fn remove_node() {
        let mut tree = LayoutTree::new();
        let n = tree.add_leaf(&NodeStyle::default()).unwrap();
        assert_eq!(tree.node_count(), 1);
        tree.remove(n).unwrap();
        assert_eq!(tree.node_count(), 0);
    }

    // --- Flexbox layout tests ---

    #[test]
    fn flex_row_children_side_by_side() {
        let mut tree = LayoutTree::new();
        let child_style = NodeStyle {
            width: Dim::Cells(10.0),
            height: Dim::Cells(5.0),
            ..NodeStyle::default()
        };
        let a = tree.add_leaf(&child_style).unwrap();
        let b = tree.add_leaf(&child_style).unwrap();

        let parent_style = NodeStyle {
            display: DisplayMode::Flex(FlexStyle {
                direction: FlexDir::Row,
                ..FlexStyle::default()
            }),
            width: Dim::Cells(80.0),
            height: Dim::Cells(24.0),
            ..NodeStyle::default()
        };
        let root = tree.add_node(&parent_style, &[a, b]).unwrap();

        tree.compute(root, 80.0, 24.0).unwrap();

        let la = tree.get_layout(a).unwrap();
        let lb = tree.get_layout(b).unwrap();

        // Both children should have the same height.
        assert!((la.height - 5.0).abs() < f32::EPSILON);
        assert!((lb.height - 5.0).abs() < f32::EPSILON);
        // a starts at x=0, b starts at x=10 (width of a).
        assert!((la.x - 0.0).abs() < f32::EPSILON);
        assert!((lb.x - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn flex_column_children_stacked() {
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

        let la = tree.get_layout(a).unwrap();
        let lb = tree.get_layout(b).unwrap();

        // Children stacked vertically.
        assert!((la.y - 0.0).abs() < f32::EPSILON);
        assert!((lb.y - 5.0).abs() < f32::EPSILON);
        // Both at x=0.
        assert!((la.x - 0.0).abs() < f32::EPSILON);
        assert!((lb.x - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn flex_grow_distributes_space() {
        let mut tree = LayoutTree::new();
        let child_a = NodeStyle {
            display: DisplayMode::Flex(FlexStyle {
                grow: 1.0,
                ..FlexStyle::default()
            }),
            height: Dim::Cells(5.0),
            ..NodeStyle::default()
        };
        let child_b = NodeStyle {
            display: DisplayMode::Flex(FlexStyle {
                grow: 2.0,
                ..FlexStyle::default()
            }),
            height: Dim::Cells(5.0),
            ..NodeStyle::default()
        };
        let a = tree.add_leaf(&child_a).unwrap();
        let b = tree.add_leaf(&child_b).unwrap();

        let parent_style = NodeStyle {
            display: DisplayMode::Flex(FlexStyle {
                direction: FlexDir::Row,
                ..FlexStyle::default()
            }),
            width: Dim::Cells(90.0),
            height: Dim::Cells(24.0),
            ..NodeStyle::default()
        };
        let root = tree.add_node(&parent_style, &[a, b]).unwrap();
        tree.compute(root, 90.0, 24.0).unwrap();

        let la = tree.get_layout(a).unwrap();
        let lb = tree.get_layout(b).unwrap();

        // grow 1:2 ratio → a gets 30, b gets 60
        assert!((la.width - 30.0).abs() < 0.1);
        assert!((lb.width - 60.0).abs() < 0.1);
    }

    #[test]
    fn flex_justify_center() {
        let mut tree = LayoutTree::new();
        let child_style = NodeStyle {
            width: Dim::Cells(20.0),
            height: Dim::Cells(5.0),
            ..NodeStyle::default()
        };
        let a = tree.add_leaf(&child_style).unwrap();

        let parent_style = NodeStyle {
            display: DisplayMode::Flex(FlexStyle {
                direction: FlexDir::Row,
                justify: JustifyContent::Center,
                ..FlexStyle::default()
            }),
            width: Dim::Cells(80.0),
            height: Dim::Cells(24.0),
            ..NodeStyle::default()
        };
        let root = tree.add_node(&parent_style, &[a]).unwrap();
        tree.compute(root, 80.0, 24.0).unwrap();

        let la = tree.get_layout(a).unwrap();
        // Child should be centered: (80 - 20) / 2 = 30
        assert!((la.x - 30.0).abs() < 0.1);
    }

    #[test]
    fn flex_wrap_wraps_children() {
        let mut tree = LayoutTree::new();
        let child_style = NodeStyle {
            display: DisplayMode::Flex(FlexStyle {
                shrink: 0.0,
                ..FlexStyle::default()
            }),
            width: Dim::Cells(30.0),
            height: Dim::Cells(5.0),
            ..NodeStyle::default()
        };
        let a = tree.add_leaf(&child_style).unwrap();
        let b = tree.add_leaf(&child_style).unwrap();
        let c = tree.add_leaf(&child_style).unwrap();

        let parent_style = NodeStyle {
            display: DisplayMode::Flex(FlexStyle {
                direction: FlexDir::Row,
                wrap: Wrap::Wrap,
                ..FlexStyle::default()
            }),
            width: Dim::Cells(50.0),
            height: Dim::Cells(24.0),
            ..NodeStyle::default()
        };
        let root = tree.add_node(&parent_style, &[a, b, c]).unwrap();
        tree.compute(root, 50.0, 24.0).unwrap();

        let la = tree.get_layout(a).unwrap();
        let lb = tree.get_layout(b).unwrap();
        let lc = tree.get_layout(c).unwrap();

        // Each child is on a separate row (30 wide, container only 50).
        // All at x=0, each on successive rows.
        assert!((la.x).abs() < f32::EPSILON);
        assert!((lb.x).abs() < f32::EPSILON);
        assert!((lc.x).abs() < f32::EPSILON);
        // b and c should be below a (exact y depends on align_content stretching).
        assert!(lb.y > la.y);
        assert!(lc.y > lb.y);
        // All should maintain their declared width.
        assert!((la.width - 30.0).abs() < f32::EPSILON);
        assert!((lb.width - 30.0).abs() < f32::EPSILON);
        assert!((lc.width - 30.0).abs() < f32::EPSILON);
    }

    #[test]
    fn flex_gap_adds_spacing() {
        let mut tree = LayoutTree::new();
        let child_style = NodeStyle {
            width: Dim::Cells(10.0),
            height: Dim::Cells(5.0),
            ..NodeStyle::default()
        };
        let a = tree.add_leaf(&child_style).unwrap();
        let b = tree.add_leaf(&child_style).unwrap();

        let parent_style = NodeStyle {
            display: DisplayMode::Flex(FlexStyle {
                direction: FlexDir::Row,
                ..FlexStyle::default()
            }),
            width: Dim::Cells(80.0),
            height: Dim::Cells(24.0),
            gap: [Dim::Cells(5.0), Dim::Cells(0.0)],
            ..NodeStyle::default()
        };
        let root = tree.add_node(&parent_style, &[a, b]).unwrap();
        tree.compute(root, 80.0, 24.0).unwrap();

        let la = tree.get_layout(a).unwrap();
        let lb = tree.get_layout(b).unwrap();

        // b should start at a.width + gap = 10 + 5 = 15.
        assert!((la.x - 0.0).abs() < f32::EPSILON);
        assert!((lb.x - 15.0).abs() < f32::EPSILON);
    }

    // --- Grid layout tests ---

    #[test]
    fn grid_basic_two_columns() {
        let mut tree = LayoutTree::new();
        let child_style = NodeStyle {
            height: Dim::Cells(5.0),
            ..NodeStyle::default()
        };
        let a = tree.add_leaf(&child_style).unwrap();
        let b = tree.add_leaf(&child_style).unwrap();

        let parent_style = NodeStyle {
            display: DisplayMode::Grid(GridStyle {
                columns: vec![TrackDef::Fr(1.0), TrackDef::Fr(1.0)],
                rows: vec![TrackDef::Auto],
                ..GridStyle::default()
            }),
            width: Dim::Cells(80.0),
            height: Dim::Cells(24.0),
            ..NodeStyle::default()
        };
        let root = tree.add_node(&parent_style, &[a, b]).unwrap();
        tree.compute(root, 80.0, 24.0).unwrap();

        let la = tree.get_layout(a).unwrap();
        let lb = tree.get_layout(b).unwrap();

        // Two equal columns: each 40 wide.
        assert!((la.width - 40.0).abs() < 0.1);
        assert!((lb.width - 40.0).abs() < 0.1);
        assert!((la.x - 0.0).abs() < f32::EPSILON);
        assert!((lb.x - 40.0).abs() < 0.1);
    }

    #[test]
    fn grid_with_gap() {
        let mut tree = LayoutTree::new();
        let child_style = NodeStyle::default();
        let a = tree.add_leaf(&child_style).unwrap();
        let b = tree.add_leaf(&child_style).unwrap();

        let parent_style = NodeStyle {
            display: DisplayMode::Grid(GridStyle {
                columns: vec![TrackDef::Fr(1.0), TrackDef::Fr(1.0)],
                rows: vec![TrackDef::Auto],
                column_gap: Dim::Cells(10.0),
                ..GridStyle::default()
            }),
            width: Dim::Cells(80.0),
            height: Dim::Cells(24.0),
            ..NodeStyle::default()
        };
        let root = tree.add_node(&parent_style, &[a, b]).unwrap();
        tree.compute(root, 80.0, 24.0).unwrap();

        let la = tree.get_layout(a).unwrap();
        let lb = tree.get_layout(b).unwrap();

        // 80 - 10 (gap) = 70, split equally → 35 each.
        assert!((la.width - 35.0).abs() < 0.1);
        assert!((lb.width - 35.0).abs() < 0.1);
        // b starts at 35 + 10 (gap) = 45.
        assert!((lb.x - 45.0).abs() < 0.1);
    }

    #[test]
    fn grid_fixed_and_fr_columns() {
        let mut tree = LayoutTree::new();
        let child_style = NodeStyle::default();
        let a = tree.add_leaf(&child_style).unwrap();
        let b = tree.add_leaf(&child_style).unwrap();

        let parent_style = NodeStyle {
            display: DisplayMode::Grid(GridStyle {
                columns: vec![TrackDef::Cells(20.0), TrackDef::Fr(1.0)],
                rows: vec![TrackDef::Auto],
                ..GridStyle::default()
            }),
            width: Dim::Cells(80.0),
            height: Dim::Cells(24.0),
            ..NodeStyle::default()
        };
        let root = tree.add_node(&parent_style, &[a, b]).unwrap();
        tree.compute(root, 80.0, 24.0).unwrap();

        let la = tree.get_layout(a).unwrap();
        let lb = tree.get_layout(b).unwrap();

        // First column is fixed 20, second gets 60.
        assert!((la.width - 20.0).abs() < 0.1);
        assert!((lb.width - 60.0).abs() < 0.1);
    }

    // --- Padding and margin tests ---

    #[test]
    fn padding_insets_children() {
        let mut tree = LayoutTree::new();
        let child_style = NodeStyle {
            width: Dim::Cells(10.0),
            height: Dim::Cells(5.0),
            ..NodeStyle::default()
        };
        let a = tree.add_leaf(&child_style).unwrap();

        let parent_style = NodeStyle {
            display: DisplayMode::Flex(FlexStyle::default()),
            width: Dim::Cells(80.0),
            height: Dim::Cells(24.0),
            padding: [
                Dim::Cells(2.0),
                Dim::Cells(3.0),
                Dim::Cells(2.0),
                Dim::Cells(3.0),
            ],
            ..NodeStyle::default()
        };
        let root = tree.add_node(&parent_style, &[a]).unwrap();
        tree.compute(root, 80.0, 24.0).unwrap();

        let la = tree.get_layout(a).unwrap();
        // Child should be offset by padding (left=3, top=2).
        assert!((la.x - 3.0).abs() < f32::EPSILON);
        assert!((la.y - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn percentage_width() {
        let mut tree = LayoutTree::new();
        let child_style = NodeStyle {
            width: Dim::Percent(50.0),
            height: Dim::Cells(5.0),
            ..NodeStyle::default()
        };
        let a = tree.add_leaf(&child_style).unwrap();

        let parent_style = NodeStyle {
            display: DisplayMode::Flex(FlexStyle::default()),
            width: Dim::Cells(80.0),
            height: Dim::Cells(24.0),
            ..NodeStyle::default()
        };
        let root = tree.add_node(&parent_style, &[a]).unwrap();
        tree.compute(root, 80.0, 24.0).unwrap();

        let la = tree.get_layout(a).unwrap();
        assert!((la.width - 40.0).abs() < 0.1);
    }

    // --- Style update test ---

    #[test]
    fn set_style_updates_layout() {
        let mut tree = LayoutTree::new();
        let child_style = NodeStyle {
            width: Dim::Cells(10.0),
            height: Dim::Cells(5.0),
            ..NodeStyle::default()
        };
        let a = tree.add_leaf(&child_style).unwrap();

        let parent_style = NodeStyle {
            display: DisplayMode::Flex(FlexStyle::default()),
            width: Dim::Cells(80.0),
            height: Dim::Cells(24.0),
            ..NodeStyle::default()
        };
        let root = tree.add_node(&parent_style, &[a]).unwrap();
        tree.compute(root, 80.0, 24.0).unwrap();

        let la = tree.get_layout(a).unwrap();
        assert!((la.width - 10.0).abs() < f32::EPSILON);

        // Update width.
        let new_style = NodeStyle {
            width: Dim::Cells(30.0),
            height: Dim::Cells(5.0),
            ..NodeStyle::default()
        };
        tree.set_style(a, &new_style).unwrap();
        tree.compute(root, 80.0, 24.0).unwrap();

        let la2 = tree.get_layout(a).unwrap();
        assert!((la2.width - 30.0).abs() < f32::EPSILON);
    }

    // --- Add child test ---

    #[test]
    fn add_child_dynamically() {
        let mut tree = LayoutTree::new();
        let child_style = NodeStyle {
            width: Dim::Cells(10.0),
            height: Dim::Cells(5.0),
            ..NodeStyle::default()
        };

        let parent_style = NodeStyle {
            display: DisplayMode::Flex(FlexStyle::default()),
            width: Dim::Cells(80.0),
            height: Dim::Cells(24.0),
            ..NodeStyle::default()
        };
        let root = tree.add_node(&parent_style, &[]).unwrap();
        let a = tree.add_leaf(&child_style).unwrap();
        tree.add_child(root, a).unwrap();

        tree.compute(root, 80.0, 24.0).unwrap();
        let la = tree.get_layout(a).unwrap();
        assert!((la.width - 10.0).abs() < f32::EPSILON);
    }
}
