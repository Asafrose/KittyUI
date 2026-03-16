//! Focus management system for the layout tree.
//!
//! Tracks which node currently has focus, supports tab-order navigation
//! (tab / shift+tab), `tabIndex`-style ordering, and focus trapping
//! within subtrees (for modals and dialogs).
//!
//! # Tab index semantics
//!
//! - **Negative** (`-1`): Node is focusable programmatically but skipped
//!   during sequential tab navigation.
//! - **Zero** (`0`): Node participates in tab order at its natural position
//!   (depth-first tree order).
//! - **Positive** (`1`, `2`, …): Node is visited before zero-index nodes,
//!   in ascending order of its value.

use std::collections::HashMap;

use crate::layout::{LayoutNodeId, LayoutTree};

// ---------------------------------------------------------------------------
// Focus event
// ---------------------------------------------------------------------------

/// Events emitted by the focus system.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FocusEvent {
    /// A node received focus.
    Focus(LayoutNodeId),
    /// A node lost focus.
    Blur(LayoutNodeId),
}

// ---------------------------------------------------------------------------
// Focus metadata
// ---------------------------------------------------------------------------

/// Per-node focus configuration.
#[derive(Clone, Copy, Debug, Default)]
pub struct FocusMeta {
    /// Tab index controlling sequential navigation order.
    ///
    /// - Negative: focusable but not in tab sequence.
    /// - Zero: in tab sequence at tree position.
    /// - Positive: in tab sequence, ordered by value (lower first).
    pub tab_index: i32,
}

// ---------------------------------------------------------------------------
// FocusManager
// ---------------------------------------------------------------------------

/// Manages focus state, tab navigation, and focus trapping.
pub struct FocusManager {
    /// Per-node focus metadata.
    meta: HashMap<LayoutNodeId, FocusMeta>,
    /// The node that currently has focus, if any.
    focused: Option<LayoutNodeId>,
    /// Root of the subtree that traps focus, if any.
    trap_root: Option<LayoutNodeId>,
}

impl FocusManager {
    /// Create a new focus manager with no focused node.
    #[must_use]
    pub fn new() -> Self {
        Self {
            meta: HashMap::new(),
            focused: None,
            trap_root: None,
        }
    }

    /// Return the currently focused node, if any.
    #[must_use]
    pub fn focused(&self) -> Option<LayoutNodeId> {
        self.focused
    }

    /// Register a node as focusable with the given metadata.
    pub fn set_meta(&mut self, node: LayoutNodeId, meta: FocusMeta) {
        self.meta.insert(node, meta);
    }

    /// Remove focus metadata for a node.
    ///
    /// If the removed node was focused, focus is cleared and a `Blur`
    /// event is returned.
    pub fn remove_meta(&mut self, node: LayoutNodeId) -> Option<FocusEvent> {
        self.meta.remove(&node);
        if self.focused == Some(node) {
            self.focused = None;
            return Some(FocusEvent::Blur(node));
        }
        None
    }

    /// Focus a specific node by ID.
    ///
    /// The node must have focus metadata registered. Returns the events
    /// produced (blur on the old node, focus on the new node).
    pub fn focus_node(&mut self, node: LayoutNodeId) -> Vec<FocusEvent> {
        if !self.meta.contains_key(&node) {
            return Vec::new();
        }
        let mut events = Vec::new();
        if let Some(prev) = self.focused {
            if prev == node {
                return events; // already focused
            }
            events.push(FocusEvent::Blur(prev));
        }
        self.focused = Some(node);
        events.push(FocusEvent::Focus(node));
        events
    }

    /// Blur the given node. If it is currently focused, focus is cleared.
    ///
    /// Returns a `Blur` event if the node was indeed focused.
    pub fn blur_node(&mut self, node: LayoutNodeId) -> Option<FocusEvent> {
        if self.focused == Some(node) {
            self.focused = None;
            Some(FocusEvent::Blur(node))
        } else {
            None
        }
    }

    /// Clear focus entirely.
    ///
    /// Returns a `Blur` event if a node was focused.
    pub fn blur(&mut self) -> Option<FocusEvent> {
        self.focused.take().map(FocusEvent::Blur)
    }

    // -----------------------------------------------------------------------
    // Focus trapping
    // -----------------------------------------------------------------------

    /// Confine tab navigation to the subtree rooted at `root`.
    ///
    /// While a trap is active, `focus_next` / `focus_prev` only visit
    /// nodes that are descendants of (or equal to) the trap root.
    pub fn set_trap(&mut self, root: LayoutNodeId) {
        self.trap_root = Some(root);
    }

    /// Remove the focus trap, restoring full-tree navigation.
    pub fn clear_trap(&mut self) {
        self.trap_root = None;
    }

    /// Return the current trap root, if any.
    #[must_use]
    pub fn trap_root(&self) -> Option<LayoutNodeId> {
        self.trap_root
    }

    // -----------------------------------------------------------------------
    // Tab navigation
    // -----------------------------------------------------------------------

    /// Move focus to the next node in tab order.
    ///
    /// Returns the focus/blur events produced. If no node is focused,
    /// focuses the first node in order.
    ///
    /// # Errors
    ///
    /// Returns a `FocusError` if the layout tree cannot be traversed.
    pub fn focus_next(
        &mut self,
        tree: &LayoutTree,
        root: LayoutNodeId,
    ) -> Result<Vec<FocusEvent>, FocusError> {
        let nav_root = self.trap_root.unwrap_or(root);
        let order = self.build_tab_order(tree, nav_root)?;
        if order.is_empty() {
            return Ok(Vec::new());
        }
        Ok(self.move_focus(&order, Direction::Forward))
    }

    /// Move focus to the previous node in tab order (shift+tab).
    ///
    /// # Errors
    ///
    /// Returns a `FocusError` if the layout tree cannot be traversed.
    pub fn focus_prev(
        &mut self,
        tree: &LayoutTree,
        root: LayoutNodeId,
    ) -> Result<Vec<FocusEvent>, FocusError> {
        let nav_root = self.trap_root.unwrap_or(root);
        let order = self.build_tab_order(tree, nav_root)?;
        if order.is_empty() {
            return Ok(Vec::new());
        }
        Ok(self.move_focus(&order, Direction::Backward))
    }

    /// Build the sequential tab order for a subtree.
    ///
    /// Nodes with positive `tab_index` come first (sorted ascending),
    /// followed by nodes with `tab_index == 0` in tree order.
    /// Nodes with negative `tab_index` are excluded.
    fn build_tab_order(
        &self,
        tree: &LayoutTree,
        root: LayoutNodeId,
    ) -> Result<Vec<LayoutNodeId>, FocusError> {
        let mut tree_order = Vec::new();
        Self::collect_dfs(tree, root, &mut tree_order)?;

        let mut positive: Vec<(i32, usize, LayoutNodeId)> = Vec::new();
        let mut zero: Vec<LayoutNodeId> = Vec::new();

        for (idx, &node) in tree_order.iter().enumerate() {
            if let Some(meta) = self.meta.get(&node) {
                if meta.tab_index > 0 {
                    positive.push((meta.tab_index, idx, node));
                } else if meta.tab_index == 0 {
                    zero.push(node);
                }
                // negative tab_index: skip
            }
        }

        // Sort positive by (tab_index, tree_order) for stability.
        positive.sort_by_key(|(ti, idx, _)| (*ti, *idx));

        let mut order: Vec<LayoutNodeId> = positive.into_iter().map(|(_, _, n)| n).collect();
        order.extend(zero);
        Ok(order)
    }

    /// Depth-first traversal collecting all nodes.
    fn collect_dfs(
        tree: &LayoutTree,
        node: LayoutNodeId,
        out: &mut Vec<LayoutNodeId>,
    ) -> Result<(), FocusError> {
        out.push(node);
        let children = tree.children(node).map_err(|_| FocusError::LayoutError)?;
        for child in children {
            Self::collect_dfs(tree, child, out)?;
        }
        Ok(())
    }

    /// Move focus forward or backward within the given order, wrapping around.
    fn move_focus(&mut self, order: &[LayoutNodeId], dir: Direction) -> Vec<FocusEvent> {
        let current_idx = self
            .focused
            .and_then(|f| order.iter().position(|&n| n == f));

        let next_idx = match current_idx {
            Some(idx) => match dir {
                Direction::Forward => (idx + 1) % order.len(),
                Direction::Backward => {
                    if idx == 0 {
                        order.len() - 1
                    } else {
                        idx - 1
                    }
                }
            },
            None => match dir {
                Direction::Forward => 0,
                Direction::Backward => order.len() - 1,
            },
        };

        let next_node = order[next_idx];
        self.focus_node(next_node)
    }
}

impl Default for FocusManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Direction for tab navigation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Direction {
    Forward,
    Backward,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors that can occur during focus operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FocusError {
    /// Could not read layout data from the tree.
    LayoutError,
}

impl std::fmt::Display for FocusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LayoutError => write!(f, "failed to read layout data"),
        }
    }
}

impl std::error::Error for FocusError {}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{Dim, DisplayMode, FlexDir, FlexStyle, NodeStyle};

    /// Helper: build a simple tree with root and N leaf children.
    fn setup_tree(n: usize) -> (LayoutTree, LayoutNodeId, Vec<LayoutNodeId>) {
        let mut tree = LayoutTree::new();
        let child_style = NodeStyle {
            width: Dim::Cells(10.0),
            height: Dim::Cells(5.0),
            ..NodeStyle::default()
        };

        let mut children = Vec::new();
        for _ in 0..n {
            let node = tree.add_leaf(&child_style).ok();
            if let Some(node) = node {
                children.push(node);
            }
        }

        let parent_style = NodeStyle {
            display: DisplayMode::Flex(FlexStyle {
                direction: FlexDir::Column,
                ..FlexStyle::default()
            }),
            width: Dim::Cells(80.0),
            height: Dim::Cells(24.0),
            ..NodeStyle::default()
        };
        let root = tree.add_node(&parent_style, &children).ok();
        let root = root.unwrap_or_else(|| panic!("failed to create root"));
        tree.compute(root, 80.0, 24.0)
            .unwrap_or_else(|e| panic!("compute failed: {e}"));

        (tree, root, children)
    }

    // -- Basic focus/blur --

    #[test]
    fn initially_no_focus() {
        let fm = FocusManager::new();
        assert!(fm.focused().is_none());
    }

    #[test]
    fn focus_node_sets_focus() {
        let (_tree, _root, children) = setup_tree(3);
        let mut fm = FocusManager::new();
        fm.set_meta(children[0], FocusMeta::default());

        let events = fm.focus_node(children[0]);
        assert_eq!(fm.focused(), Some(children[0]));
        assert_eq!(events, vec![FocusEvent::Focus(children[0])]);
    }

    #[test]
    fn focus_node_emits_blur_on_previous() {
        let (_tree, _root, children) = setup_tree(3);
        let mut fm = FocusManager::new();
        fm.set_meta(children[0], FocusMeta::default());
        fm.set_meta(children[1], FocusMeta::default());

        fm.focus_node(children[0]);
        let events = fm.focus_node(children[1]);
        assert_eq!(
            events,
            vec![
                FocusEvent::Blur(children[0]),
                FocusEvent::Focus(children[1]),
            ]
        );
    }

    #[test]
    fn focus_same_node_is_noop() {
        let (_tree, _root, children) = setup_tree(3);
        let mut fm = FocusManager::new();
        fm.set_meta(children[0], FocusMeta::default());

        fm.focus_node(children[0]);
        let events = fm.focus_node(children[0]);
        assert!(events.is_empty());
        assert_eq!(fm.focused(), Some(children[0]));
    }

    #[test]
    fn focus_unregistered_node_is_noop() {
        let (_tree, _root, children) = setup_tree(3);
        let mut fm = FocusManager::new();
        // children[0] has no meta
        let events = fm.focus_node(children[0]);
        assert!(events.is_empty());
        assert!(fm.focused().is_none());
    }

    #[test]
    fn blur_node_clears_focus() {
        let (_tree, _root, children) = setup_tree(3);
        let mut fm = FocusManager::new();
        fm.set_meta(children[0], FocusMeta::default());
        fm.focus_node(children[0]);

        let event = fm.blur_node(children[0]);
        assert_eq!(event, Some(FocusEvent::Blur(children[0])));
        assert!(fm.focused().is_none());
    }

    #[test]
    fn blur_wrong_node_is_noop() {
        let (_tree, _root, children) = setup_tree(3);
        let mut fm = FocusManager::new();
        fm.set_meta(children[0], FocusMeta::default());
        fm.set_meta(children[1], FocusMeta::default());
        fm.focus_node(children[0]);

        let event = fm.blur_node(children[1]);
        assert!(event.is_none());
        assert_eq!(fm.focused(), Some(children[0]));
    }

    #[test]
    fn blur_clears_any_focus() {
        let (_tree, _root, children) = setup_tree(3);
        let mut fm = FocusManager::new();
        fm.set_meta(children[0], FocusMeta::default());
        fm.focus_node(children[0]);

        let event = fm.blur();
        assert_eq!(event, Some(FocusEvent::Blur(children[0])));
        assert!(fm.focused().is_none());
    }

    #[test]
    fn blur_when_nothing_focused() {
        let mut fm = FocusManager::new();
        assert!(fm.blur().is_none());
    }

    // -- Remove meta --

    #[test]
    fn remove_meta_blurs_focused_node() {
        let (_tree, _root, children) = setup_tree(3);
        let mut fm = FocusManager::new();
        fm.set_meta(children[0], FocusMeta::default());
        fm.focus_node(children[0]);

        let event = fm.remove_meta(children[0]);
        assert_eq!(event, Some(FocusEvent::Blur(children[0])));
        assert!(fm.focused().is_none());
    }

    #[test]
    fn remove_meta_of_unfocused_node_is_quiet() {
        let (_tree, _root, children) = setup_tree(3);
        let mut fm = FocusManager::new();
        fm.set_meta(children[0], FocusMeta::default());

        let event = fm.remove_meta(children[0]);
        assert!(event.is_none());
    }

    // -- Tab navigation (tab_index = 0) --

    #[test]
    fn focus_next_with_no_focus_goes_to_first() {
        let (tree, root, children) = setup_tree(3);
        let mut fm = FocusManager::new();
        for &c in &children {
            fm.set_meta(c, FocusMeta::default());
        }

        let events = fm.focus_next(&tree, root).unwrap_or_default();
        assert_eq!(fm.focused(), Some(children[0]));
        assert_eq!(events, vec![FocusEvent::Focus(children[0])]);
    }

    #[test]
    fn focus_next_advances_sequentially() {
        let (tree, root, children) = setup_tree(3);
        let mut fm = FocusManager::new();
        for &c in &children {
            fm.set_meta(c, FocusMeta::default());
        }

        fm.focus_next(&tree, root).unwrap_or_default();
        assert_eq!(fm.focused(), Some(children[0]));

        fm.focus_next(&tree, root).unwrap_or_default();
        assert_eq!(fm.focused(), Some(children[1]));

        fm.focus_next(&tree, root).unwrap_or_default();
        assert_eq!(fm.focused(), Some(children[2]));
    }

    #[test]
    fn focus_next_wraps_around() {
        let (tree, root, children) = setup_tree(3);
        let mut fm = FocusManager::new();
        for &c in &children {
            fm.set_meta(c, FocusMeta::default());
        }

        // Go to last
        fm.focus_node(children[2]);
        let events = fm.focus_next(&tree, root).unwrap_or_default();
        assert_eq!(fm.focused(), Some(children[0]));
        assert_eq!(
            events,
            vec![
                FocusEvent::Blur(children[2]),
                FocusEvent::Focus(children[0]),
            ]
        );
    }

    #[test]
    fn focus_prev_with_no_focus_goes_to_last() {
        let (tree, root, children) = setup_tree(3);
        let mut fm = FocusManager::new();
        for &c in &children {
            fm.set_meta(c, FocusMeta::default());
        }

        let events = fm.focus_prev(&tree, root).unwrap_or_default();
        assert_eq!(fm.focused(), Some(children[2]));
        assert_eq!(events, vec![FocusEvent::Focus(children[2])]);
    }

    #[test]
    fn focus_prev_goes_backwards() {
        let (tree, root, children) = setup_tree(3);
        let mut fm = FocusManager::new();
        for &c in &children {
            fm.set_meta(c, FocusMeta::default());
        }

        fm.focus_node(children[2]);
        fm.focus_prev(&tree, root).unwrap_or_default();
        assert_eq!(fm.focused(), Some(children[1]));

        fm.focus_prev(&tree, root).unwrap_or_default();
        assert_eq!(fm.focused(), Some(children[0]));
    }

    #[test]
    fn focus_prev_wraps_around() {
        let (tree, root, children) = setup_tree(3);
        let mut fm = FocusManager::new();
        for &c in &children {
            fm.set_meta(c, FocusMeta::default());
        }

        fm.focus_node(children[0]);
        fm.focus_prev(&tree, root).unwrap_or_default();
        assert_eq!(fm.focused(), Some(children[2]));
    }

    // -- Tab index ordering --

    #[test]
    fn positive_tab_index_comes_before_zero() {
        let (tree, root, children) = setup_tree(3);
        let mut fm = FocusManager::new();
        // children[0]: tab_index=0, children[1]: tab_index=1, children[2]: tab_index=0
        fm.set_meta(children[0], FocusMeta { tab_index: 0 });
        fm.set_meta(children[1], FocusMeta { tab_index: 1 });
        fm.set_meta(children[2], FocusMeta { tab_index: 0 });

        // Expected order: children[1] (tab_index=1), children[0] (0), children[2] (0)
        fm.focus_next(&tree, root).unwrap_or_default();
        assert_eq!(fm.focused(), Some(children[1]));

        fm.focus_next(&tree, root).unwrap_or_default();
        assert_eq!(fm.focused(), Some(children[0]));

        fm.focus_next(&tree, root).unwrap_or_default();
        assert_eq!(fm.focused(), Some(children[2]));
    }

    #[test]
    fn positive_tab_indices_sorted_ascending() {
        let (tree, root, children) = setup_tree(3);
        let mut fm = FocusManager::new();
        fm.set_meta(children[0], FocusMeta { tab_index: 3 });
        fm.set_meta(children[1], FocusMeta { tab_index: 1 });
        fm.set_meta(children[2], FocusMeta { tab_index: 2 });

        // Expected order: children[1] (1), children[2] (2), children[0] (3)
        fm.focus_next(&tree, root).unwrap_or_default();
        assert_eq!(fm.focused(), Some(children[1]));

        fm.focus_next(&tree, root).unwrap_or_default();
        assert_eq!(fm.focused(), Some(children[2]));

        fm.focus_next(&tree, root).unwrap_or_default();
        assert_eq!(fm.focused(), Some(children[0]));
    }

    #[test]
    fn negative_tab_index_excluded_from_tab_order() {
        let (tree, root, children) = setup_tree(3);
        let mut fm = FocusManager::new();
        fm.set_meta(children[0], FocusMeta { tab_index: 0 });
        fm.set_meta(children[1], FocusMeta { tab_index: -1 });
        fm.set_meta(children[2], FocusMeta { tab_index: 0 });

        // children[1] should be skipped in tab navigation
        fm.focus_next(&tree, root).unwrap_or_default();
        assert_eq!(fm.focused(), Some(children[0]));

        fm.focus_next(&tree, root).unwrap_or_default();
        assert_eq!(fm.focused(), Some(children[2]));

        // Wraps back to children[0], skipping children[1]
        fm.focus_next(&tree, root).unwrap_or_default();
        assert_eq!(fm.focused(), Some(children[0]));
    }

    #[test]
    fn negative_tab_index_still_focusable_programmatically() {
        let (_tree, _root, children) = setup_tree(3);
        let mut fm = FocusManager::new();
        fm.set_meta(children[0], FocusMeta { tab_index: -1 });

        let events = fm.focus_node(children[0]);
        assert_eq!(fm.focused(), Some(children[0]));
        assert_eq!(events, vec![FocusEvent::Focus(children[0])]);
    }

    // -- Focus trapping --

    #[test]
    fn focus_trap_confines_navigation() {
        // Build a tree with root -> [container -> [a, b], c]
        let mut tree = LayoutTree::new();
        let leaf_style = NodeStyle {
            width: Dim::Cells(10.0),
            height: Dim::Cells(5.0),
            ..NodeStyle::default()
        };
        let a = tree
            .add_leaf(&leaf_style)
            .ok()
            .unwrap_or_else(|| panic!("add_leaf"));
        let b = tree
            .add_leaf(&leaf_style)
            .ok()
            .unwrap_or_else(|| panic!("add_leaf"));
        let c = tree
            .add_leaf(&leaf_style)
            .ok()
            .unwrap_or_else(|| panic!("add_leaf"));

        let container_style = NodeStyle {
            display: DisplayMode::Flex(FlexStyle {
                direction: FlexDir::Column,
                ..FlexStyle::default()
            }),
            width: Dim::Cells(40.0),
            height: Dim::Cells(20.0),
            ..NodeStyle::default()
        };
        let container = tree
            .add_node(&container_style, &[a, b])
            .ok()
            .unwrap_or_else(|| panic!("add_node"));

        let root_style = NodeStyle {
            display: DisplayMode::Flex(FlexStyle {
                direction: FlexDir::Column,
                ..FlexStyle::default()
            }),
            width: Dim::Cells(80.0),
            height: Dim::Cells(24.0),
            ..NodeStyle::default()
        };
        let root = tree
            .add_node(&root_style, &[container, c])
            .ok()
            .unwrap_or_else(|| panic!("add_node"));
        tree.compute(root, 80.0, 24.0)
            .unwrap_or_else(|e| panic!("compute: {e}"));

        let mut fm = FocusManager::new();
        fm.set_meta(a, FocusMeta::default());
        fm.set_meta(b, FocusMeta::default());
        fm.set_meta(c, FocusMeta::default());

        // Trap focus within container (a, b only)
        fm.set_trap(container);

        fm.focus_next(&tree, root).unwrap_or_default();
        assert_eq!(fm.focused(), Some(a));

        fm.focus_next(&tree, root).unwrap_or_default();
        assert_eq!(fm.focused(), Some(b));

        // Should wrap within trap, NOT go to c
        fm.focus_next(&tree, root).unwrap_or_default();
        assert_eq!(fm.focused(), Some(a));
    }

    #[test]
    fn focus_trap_prev_wraps_within_trap() {
        let mut tree = LayoutTree::new();
        let leaf_style = NodeStyle {
            width: Dim::Cells(10.0),
            height: Dim::Cells(5.0),
            ..NodeStyle::default()
        };
        let a = tree
            .add_leaf(&leaf_style)
            .ok()
            .unwrap_or_else(|| panic!("add_leaf"));
        let b = tree
            .add_leaf(&leaf_style)
            .ok()
            .unwrap_or_else(|| panic!("add_leaf"));
        let c = tree
            .add_leaf(&leaf_style)
            .ok()
            .unwrap_or_else(|| panic!("add_leaf"));

        let container_style = NodeStyle {
            display: DisplayMode::Flex(FlexStyle {
                direction: FlexDir::Column,
                ..FlexStyle::default()
            }),
            width: Dim::Cells(40.0),
            height: Dim::Cells(20.0),
            ..NodeStyle::default()
        };
        let container = tree
            .add_node(&container_style, &[a, b])
            .ok()
            .unwrap_or_else(|| panic!("add_node"));

        let root_style = NodeStyle {
            display: DisplayMode::Flex(FlexStyle {
                direction: FlexDir::Column,
                ..FlexStyle::default()
            }),
            width: Dim::Cells(80.0),
            height: Dim::Cells(24.0),
            ..NodeStyle::default()
        };
        let root = tree
            .add_node(&root_style, &[container, c])
            .ok()
            .unwrap_or_else(|| panic!("add_node"));
        tree.compute(root, 80.0, 24.0)
            .unwrap_or_else(|e| panic!("compute: {e}"));

        let mut fm = FocusManager::new();
        fm.set_meta(a, FocusMeta::default());
        fm.set_meta(b, FocusMeta::default());
        fm.set_meta(c, FocusMeta::default());

        fm.set_trap(container);
        fm.focus_node(a);

        fm.focus_prev(&tree, root).unwrap_or_default();
        assert_eq!(fm.focused(), Some(b)); // wraps to last in trap
    }

    #[test]
    fn clear_trap_restores_full_navigation() {
        let (tree, root, children) = setup_tree(3);
        let mut fm = FocusManager::new();
        for &c in &children {
            fm.set_meta(c, FocusMeta::default());
        }

        // The root itself is the trap (all children are under root)
        // so let's just test clear_trap returns None
        fm.set_trap(root);
        assert_eq!(fm.trap_root(), Some(root));

        fm.clear_trap();
        assert!(fm.trap_root().is_none());

        // Navigation should work across all children
        fm.focus_next(&tree, root).unwrap_or_default();
        assert_eq!(fm.focused(), Some(children[0]));

        fm.focus_next(&tree, root).unwrap_or_default();
        assert_eq!(fm.focused(), Some(children[1]));

        fm.focus_next(&tree, root).unwrap_or_default();
        assert_eq!(fm.focused(), Some(children[2]));
    }

    // -- Empty / edge cases --

    #[test]
    fn focus_next_with_no_focusable_nodes() {
        let (tree, root, _children) = setup_tree(3);
        let mut fm = FocusManager::new();
        // No meta registered

        let events = fm.focus_next(&tree, root).unwrap_or_default();
        assert!(events.is_empty());
        assert!(fm.focused().is_none());
    }

    #[test]
    fn focus_next_single_node_stays() {
        let (tree, root, children) = setup_tree(1);
        let mut fm = FocusManager::new();
        fm.set_meta(children[0], FocusMeta::default());

        fm.focus_next(&tree, root).unwrap_or_default();
        assert_eq!(fm.focused(), Some(children[0]));

        // Next again wraps to same node — no blur/focus since already focused
        let events = fm.focus_next(&tree, root).unwrap_or_default();
        assert!(events.is_empty());
        assert_eq!(fm.focused(), Some(children[0]));
    }

    #[test]
    fn focus_prev_single_node_stays() {
        let (tree, root, children) = setup_tree(1);
        let mut fm = FocusManager::new();
        fm.set_meta(children[0], FocusMeta::default());

        fm.focus_prev(&tree, root).unwrap_or_default();
        assert_eq!(fm.focused(), Some(children[0]));

        let events = fm.focus_prev(&tree, root).unwrap_or_default();
        assert!(events.is_empty());
    }

    // -- Deeply nested tree --

    #[test]
    fn tab_order_respects_depth_first() {
        // root -> [container1 -> [a, b], container2 -> [c, d]]
        let mut tree = LayoutTree::new();
        let leaf_style = NodeStyle {
            width: Dim::Cells(10.0),
            height: Dim::Cells(5.0),
            ..NodeStyle::default()
        };
        let a = tree
            .add_leaf(&leaf_style)
            .ok()
            .unwrap_or_else(|| panic!("fail"));
        let b = tree
            .add_leaf(&leaf_style)
            .ok()
            .unwrap_or_else(|| panic!("fail"));
        let c = tree
            .add_leaf(&leaf_style)
            .ok()
            .unwrap_or_else(|| panic!("fail"));
        let d = tree
            .add_leaf(&leaf_style)
            .ok()
            .unwrap_or_else(|| panic!("fail"));

        let container_style = NodeStyle {
            display: DisplayMode::Flex(FlexStyle {
                direction: FlexDir::Column,
                ..FlexStyle::default()
            }),
            width: Dim::Cells(40.0),
            height: Dim::Cells(12.0),
            ..NodeStyle::default()
        };
        let c1 = tree
            .add_node(&container_style, &[a, b])
            .ok()
            .unwrap_or_else(|| panic!("fail"));
        let c2 = tree
            .add_node(&container_style, &[c, d])
            .ok()
            .unwrap_or_else(|| panic!("fail"));

        let root_style = NodeStyle {
            display: DisplayMode::Flex(FlexStyle {
                direction: FlexDir::Column,
                ..FlexStyle::default()
            }),
            width: Dim::Cells(80.0),
            height: Dim::Cells(24.0),
            ..NodeStyle::default()
        };
        let root = tree
            .add_node(&root_style, &[c1, c2])
            .ok()
            .unwrap_or_else(|| panic!("fail"));
        tree.compute(root, 80.0, 24.0)
            .unwrap_or_else(|e| panic!("compute: {e}"));

        let mut fm = FocusManager::new();
        fm.set_meta(a, FocusMeta::default());
        fm.set_meta(b, FocusMeta::default());
        fm.set_meta(c, FocusMeta::default());
        fm.set_meta(d, FocusMeta::default());

        // DFS order: root, c1, a, b, c2, c, d
        // Only a, b, c, d have meta → order: a, b, c, d
        fm.focus_next(&tree, root).unwrap_or_default();
        assert_eq!(fm.focused(), Some(a));

        fm.focus_next(&tree, root).unwrap_or_default();
        assert_eq!(fm.focused(), Some(b));

        fm.focus_next(&tree, root).unwrap_or_default();
        assert_eq!(fm.focused(), Some(c));

        fm.focus_next(&tree, root).unwrap_or_default();
        assert_eq!(fm.focused(), Some(d));
    }

    // -- Mixed tab indices in nested tree --

    #[test]
    fn mixed_tab_indices_nested() {
        let mut tree = LayoutTree::new();
        let leaf_style = NodeStyle {
            width: Dim::Cells(10.0),
            height: Dim::Cells(5.0),
            ..NodeStyle::default()
        };
        let a = tree
            .add_leaf(&leaf_style)
            .ok()
            .unwrap_or_else(|| panic!("fail"));
        let b = tree
            .add_leaf(&leaf_style)
            .ok()
            .unwrap_or_else(|| panic!("fail"));
        let c = tree
            .add_leaf(&leaf_style)
            .ok()
            .unwrap_or_else(|| panic!("fail"));

        let root_style = NodeStyle {
            display: DisplayMode::Flex(FlexStyle {
                direction: FlexDir::Column,
                ..FlexStyle::default()
            }),
            width: Dim::Cells(80.0),
            height: Dim::Cells(24.0),
            ..NodeStyle::default()
        };
        let root = tree
            .add_node(&root_style, &[a, b, c])
            .ok()
            .unwrap_or_else(|| panic!("fail"));
        tree.compute(root, 80.0, 24.0)
            .unwrap_or_else(|e| panic!("compute: {e}"));

        let mut fm = FocusManager::new();
        fm.set_meta(a, FocusMeta { tab_index: 0 });
        fm.set_meta(b, FocusMeta { tab_index: 2 });
        fm.set_meta(c, FocusMeta { tab_index: -1 });

        // Order: b (2), a (0). c is skipped (-1).
        fm.focus_next(&tree, root).unwrap_or_default();
        assert_eq!(fm.focused(), Some(b));

        fm.focus_next(&tree, root).unwrap_or_default();
        assert_eq!(fm.focused(), Some(a));

        // Wraps back to b
        fm.focus_next(&tree, root).unwrap_or_default();
        assert_eq!(fm.focused(), Some(b));
    }

    // -- Default trait --

    #[test]
    fn focus_manager_default_works() {
        let fm = FocusManager::default();
        assert!(fm.focused().is_none());
        assert!(fm.trap_root().is_none());
    }

    #[test]
    fn focus_meta_default() {
        let meta = FocusMeta::default();
        assert_eq!(meta.tab_index, 0);
    }

    // -- Error display --

    #[test]
    fn focus_error_display() {
        assert_eq!(
            FocusError::LayoutError.to_string(),
            "failed to read layout data"
        );
    }

    // -- Focus trap with tab indices --

    #[test]
    fn focus_trap_respects_tab_index() {
        let mut tree = LayoutTree::new();
        let leaf_style = NodeStyle {
            width: Dim::Cells(10.0),
            height: Dim::Cells(5.0),
            ..NodeStyle::default()
        };
        let a = tree
            .add_leaf(&leaf_style)
            .ok()
            .unwrap_or_else(|| panic!("fail"));
        let b = tree
            .add_leaf(&leaf_style)
            .ok()
            .unwrap_or_else(|| panic!("fail"));
        let c = tree
            .add_leaf(&leaf_style)
            .ok()
            .unwrap_or_else(|| panic!("fail"));

        let container_style = NodeStyle {
            display: DisplayMode::Flex(FlexStyle {
                direction: FlexDir::Column,
                ..FlexStyle::default()
            }),
            width: Dim::Cells(40.0),
            height: Dim::Cells(20.0),
            ..NodeStyle::default()
        };
        let container = tree
            .add_node(&container_style, &[a, b])
            .ok()
            .unwrap_or_else(|| panic!("fail"));

        let root_style = NodeStyle {
            display: DisplayMode::Flex(FlexStyle {
                direction: FlexDir::Column,
                ..FlexStyle::default()
            }),
            width: Dim::Cells(80.0),
            height: Dim::Cells(24.0),
            ..NodeStyle::default()
        };
        let root = tree
            .add_node(&root_style, &[container, c])
            .ok()
            .unwrap_or_else(|| panic!("fail"));
        tree.compute(root, 80.0, 24.0)
            .unwrap_or_else(|e| panic!("compute: {e}"));

        let mut fm = FocusManager::new();
        fm.set_meta(a, FocusMeta { tab_index: 2 });
        fm.set_meta(b, FocusMeta { tab_index: 1 });
        fm.set_meta(c, FocusMeta { tab_index: 0 });

        fm.set_trap(container);

        // Within trap, order: b (1), a (2). c is outside trap.
        fm.focus_next(&tree, root).unwrap_or_default();
        assert_eq!(fm.focused(), Some(b));

        fm.focus_next(&tree, root).unwrap_or_default();
        assert_eq!(fm.focused(), Some(a));

        // Wraps within trap
        fm.focus_next(&tree, root).unwrap_or_default();
        assert_eq!(fm.focused(), Some(b));
    }

    // -- Navigation when focused node is outside tab order --

    #[test]
    fn focus_next_when_focused_not_in_order() {
        let (tree, root, children) = setup_tree(3);
        let mut fm = FocusManager::new();
        fm.set_meta(children[0], FocusMeta { tab_index: -1 });
        fm.set_meta(children[1], FocusMeta::default());
        fm.set_meta(children[2], FocusMeta::default());

        // Programmatically focus children[0] which has tab_index=-1
        fm.focus_node(children[0]);

        // focus_next should go to first in order (children[1])
        // since children[0] is not in the tab order list
        fm.focus_next(&tree, root).unwrap_or_default();
        assert_eq!(fm.focused(), Some(children[1]));
    }
}
