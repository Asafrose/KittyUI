//! Virtual and Unicode placements for text-reflow-aware image display.
//!
//! This module implements the Kitty graphics protocol's virtual placement
//! mechanism (U=1 flag) which uses Unicode placeholder characters (U+10EEEE)
//! combined with diacritics to encode image ID and cell position. Virtual
//! placements reflow with surrounding text when the terminal is resized.
//!
//! Features:
//! - Virtual placement creation with U=1 flag
//! - Unicode placeholder generation (U+10EEEE with combining diacritics)
//! - Relative placements: position child images relative to a parent placement
//! - Automatic cascade deletion of child placements

use std::collections::{HashMap, HashSet};

use crate::image_placement::{CursorMovement, ImageLayer, ImagePlacement, ScaleMode};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// The Unicode placeholder character used by Kitty for virtual placements.
/// This is the last codepoint in the Supplementary Private Use Area-B.
const PLACEHOLDER_CHAR: char = '\u{10EEEE}';

/// Diacritics used to encode values in rows/columns of Unicode placeholders.
/// The Kitty protocol uses a set of 256 combining characters from the
/// Combining Diacritical Marks block and related blocks.
/// We use U+0305..U+0304 style diacritics for the image ID encoding and
/// U+0305 (combining overline) as the base combining mark.
///
/// In the Kitty protocol, the image ID and placement ID are encoded using
/// diacritics from U+0300-U+036F (Combining Diacritical Marks). Each diacritics
/// encodes one value in the range [0, 255].
const DIACRITICS_START: u32 = 0x0305;

/// Number of usable diacritics for encoding (we use 256 values).
#[cfg(test)]
const DIACRITICS_COUNT: u32 = 256;

// ---------------------------------------------------------------------------
// Encoding helpers
// ---------------------------------------------------------------------------

/// Encode a single byte value as a combining diacritical mark.
///
/// The Kitty protocol encodes the image ID and placement ID using
/// combining characters starting at U+0305.
fn encode_diacritic(value: u8) -> char {
    // Safety: these are valid Unicode codepoints in the combining marks range
    char::from_u32(DIACRITICS_START + u32::from(value)).unwrap_or('\u{0305}')
}

/// Encode a u32 value as a sequence of up to 4 diacritics (little-endian byte order).
fn encode_id_diacritics(id: u32) -> Vec<char> {
    let bytes = id.to_le_bytes();
    // Skip trailing zero bytes to keep the output compact
    let len = 4 - bytes.iter().rev().take_while(|&&b| b == 0).count();
    let len = len.max(1); // Always emit at least one diacritic
    bytes[..len].iter().map(|&b| encode_diacritic(b)).collect()
}

/// Decode diacritics back to a u32 value (for testing/round-trip verification).
#[cfg(test)]
fn decode_id_diacritics(chars: &[char]) -> u32 {
    let mut bytes = [0u8; 4];
    for (i, &ch) in chars.iter().enumerate().take(4) {
        let val = (ch as u32).saturating_sub(DIACRITICS_START);
        if val < DIACRITICS_COUNT {
            #[allow(clippy::cast_possible_truncation)]
            let byte = val as u8;
            bytes[i] = byte;
        }
    }
    u32::from_le_bytes(bytes)
}

/// Encode a row and column index as diacritics.
///
/// Row and column are each encoded as a single byte (0-255).
#[cfg(test)]
fn encode_row_col(row: u8, col: u8) -> (char, char) {
    (encode_diacritic(row), encode_diacritic(col))
}

// ---------------------------------------------------------------------------
// Unicode placeholder generation
// ---------------------------------------------------------------------------

/// Generate the Unicode placeholder string for a virtual placement.
///
/// Each cell of the image is represented by the placeholder character (U+10EEEE)
/// followed by combining diacritics that encode:
/// 1. The image ID (3rd and 4th diacritics, little-endian)
/// 2. The placement ID (5th and 6th diacritics, if present)
/// 3. The row index (1st diacritic)
/// 4. The column index (2nd diacritic)
///
/// The result is a multi-line string where each line represents one row of cells.
#[must_use]
pub fn generate_placeholder(
    image_id: u32,
    placement_id: Option<u32>,
    rows: u32,
    cols: u32,
) -> String {
    let id_diacritics = encode_id_diacritics(image_id);
    let pid_diacritics = placement_id.map(encode_id_diacritics);

    let mut result = String::new();

    for row in 0..rows {
        if row > 0 {
            result.push('\n');
        }
        for col in 0..cols {
            result.push(PLACEHOLDER_CHAR);

            // Row diacritic
            #[allow(clippy::cast_possible_truncation)]
            result.push(encode_diacritic(row as u8));
            // Column diacritic
            #[allow(clippy::cast_possible_truncation)]
            result.push(encode_diacritic(col as u8));

            // Image ID diacritics
            for &d in &id_diacritics {
                result.push(d);
            }

            // Placement ID diacritics (if provided)
            if let Some(ref pid) = pid_diacritics {
                for &d in pid {
                    result.push(d);
                }
            }
        }
    }

    result
}

/// Check if a character is the Unicode placeholder character.
#[must_use]
pub fn is_placeholder_char(ch: char) -> bool {
    ch == PLACEHOLDER_CHAR
}

// ---------------------------------------------------------------------------
// Virtual placement (U=1 flag)
// ---------------------------------------------------------------------------

/// A virtual placement that uses Unicode placeholders for text-reflow-aware display.
///
/// Virtual placements are transmitted with the `U=1` flag in the Kitty protocol.
/// Instead of being placed at absolute cursor positions, the terminal renders
/// the image wherever the Unicode placeholder characters appear in the text flow.
#[derive(Clone, Debug, PartialEq)]
pub struct VirtualPlacement {
    /// The underlying image placement configuration.
    pub placement: ImagePlacement,
    /// Number of rows the virtual placement spans.
    pub rows: u32,
    /// Number of columns the virtual placement spans.
    pub cols: u32,
}

impl VirtualPlacement {
    /// Create a new virtual placement.
    #[must_use]
    pub fn new(image_id: u32, rows: u32, cols: u32) -> Self {
        Self {
            placement: ImagePlacement::new(image_id),
            rows: rows.max(1),
            cols: cols.max(1),
        }
    }

    /// Set the placement ID.
    #[must_use]
    pub fn with_placement_id(mut self, id: u32) -> Self {
        self.placement = self.placement.with_placement_id(id);
        self
    }

    /// Set the scaling mode.
    #[must_use]
    pub fn scale_mode(mut self, mode: ScaleMode) -> Self {
        self.placement = self.placement.scale_mode(mode);
        self
    }

    /// Set the z-index layer.
    #[must_use]
    pub fn on_layer(mut self, layer: ImageLayer) -> Self {
        self.placement = self.placement.on_layer(layer);
        self
    }

    /// Set the z-index value.
    #[must_use]
    pub fn with_z_index(mut self, z: i32) -> Self {
        self.placement = self.placement.with_z_index(z);
        self
    }

    /// Generate the Kitty protocol escape sequence to create this virtual placement.
    ///
    /// This emits the `a=p` (place) command with `U=1` to mark it as virtual.
    #[must_use]
    pub fn encode_create(&self) -> Vec<u8> {
        let mut params = Vec::new();

        params.push("a=p".to_owned());
        params.push(format!("i={}", self.placement.image_id));

        if let Some(pid) = self.placement.placement_id {
            params.push(format!("p={pid}"));
        }

        // Virtual placement flag
        params.push("U=1".to_owned());

        // Display size in cells
        params.push(format!("c={}", self.cols));
        params.push(format!("r={}", self.rows));

        // Z-index
        if self.placement.z_index != 0 {
            params.push(format!("z={}", self.placement.z_index));
        }

        // Cursor movement
        if self.placement.cursor == CursorMovement::Suppress {
            params.push("C=1".to_owned());
        }

        let param_str = params.join(",");
        format!("\x1b_G{param_str};\x1b\\").into_bytes()
    }

    /// Generate the Unicode placeholder string for this virtual placement.
    #[must_use]
    pub fn generate_placeholder(&self) -> String {
        generate_placeholder(
            self.placement.image_id,
            self.placement.placement_id,
            self.rows,
            self.cols,
        )
    }

    /// Generate the full output: the escape sequence followed by the placeholder text.
    #[must_use]
    pub fn encode_full(&self) -> Vec<u8> {
        let mut buf = self.encode_create();
        buf.extend_from_slice(self.generate_placeholder().as_bytes());
        buf
    }
}

// ---------------------------------------------------------------------------
// Relative placements
// ---------------------------------------------------------------------------

/// A relative placement positions a child image relative to a parent placement.
///
/// The child's position is expressed as an offset from the parent's top-left
/// corner in cell coordinates. When the parent reflows (because it's a virtual
/// placement), children move with it.
#[derive(Clone, Debug, PartialEq)]
pub struct RelativePlacement {
    /// The child placement.
    pub placement: ImagePlacement,
    /// Handle of the parent placement in the `VirtualPlacementManager`.
    pub parent_handle: u64,
    /// Column offset from the parent's top-left.
    pub offset_col: i32,
    /// Row offset from the parent's top-left.
    pub offset_row: i32,
    /// Number of rows this child spans.
    pub rows: u32,
    /// Number of columns this child spans.
    pub cols: u32,
}

impl RelativePlacement {
    /// Create a new relative placement.
    #[must_use]
    pub fn new(image_id: u32, parent_handle: u64, rows: u32, cols: u32) -> Self {
        Self {
            placement: ImagePlacement::new(image_id),
            parent_handle,
            offset_col: 0,
            offset_row: 0,
            rows: rows.max(1),
            cols: cols.max(1),
        }
    }

    /// Set the offset from the parent.
    #[must_use]
    pub fn offset(mut self, col: i32, row: i32) -> Self {
        self.offset_col = col;
        self.offset_row = row;
        self
    }

    /// Set the placement ID.
    #[must_use]
    pub fn with_placement_id(mut self, id: u32) -> Self {
        self.placement = self.placement.with_placement_id(id);
        self
    }

    /// Set the z-index value.
    #[must_use]
    pub fn with_z_index(mut self, z: i32) -> Self {
        self.placement = self.placement.with_z_index(z);
        self
    }

    /// Compute the absolute position given the parent's position.
    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn absolute_position(&self, parent_col: u32, parent_row: u32) -> (u32, u32) {
        let col = (i64::from(parent_col) + i64::from(self.offset_col)).max(0) as u32;
        let row = (i64::from(parent_row) + i64::from(self.offset_row)).max(0) as u32;
        (col, row)
    }

    /// Encode this relative placement as a Kitty protocol virtual placement command.
    ///
    /// The parent's position is needed to compute the absolute position for the
    /// placement command's `P` (parent placement ID) reference.
    #[must_use]
    pub fn encode_create(&self, parent_placement_id: u32) -> Vec<u8> {
        let mut params = Vec::new();

        params.push("a=p".to_owned());
        params.push(format!("i={}", self.placement.image_id));

        if let Some(pid) = self.placement.placement_id {
            params.push(format!("p={pid}"));
        }

        // Virtual placement flag
        params.push("U=1".to_owned());

        // Parent reference
        params.push(format!("P={parent_placement_id}"));

        // Offset from parent
        if self.offset_col != 0 {
            params.push(format!("H={}", self.offset_col));
        }
        if self.offset_row != 0 {
            params.push(format!("V={}", self.offset_row));
        }

        // Display size
        params.push(format!("c={}", self.cols));
        params.push(format!("r={}", self.rows));

        // Z-index
        if self.placement.z_index != 0 {
            params.push(format!("z={}", self.placement.z_index));
        }

        let param_str = params.join(",");
        format!("\x1b_G{param_str};\x1b\\").into_bytes()
    }
}

// ---------------------------------------------------------------------------
// Virtual placement manager
// ---------------------------------------------------------------------------

/// Manages virtual placements and their parent-child relationships.
///
/// Provides:
/// - Adding/removing virtual placements
/// - Adding relative (child) placements with parent references
/// - Cascade deletion: removing a parent also removes all its children
/// - Generating combined output (escape sequences + placeholder text)
#[derive(Debug, Default)]
pub struct VirtualPlacementManager {
    /// Virtual placements keyed by handle.
    placements: HashMap<u64, VirtualPlacement>,
    /// Relative (child) placements keyed by handle.
    children: HashMap<u64, RelativePlacement>,
    /// Maps parent handle -> set of child handles for cascade deletion.
    parent_children: HashMap<u64, HashSet<u64>>,
    /// Next auto-assigned handle.
    next_handle: u64,
}

impl VirtualPlacementManager {
    /// Create a new empty manager.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a virtual placement and return its handle.
    pub fn add(&mut self, placement: VirtualPlacement) -> u64 {
        let handle = self.next_handle;
        self.next_handle += 1;
        self.placements.insert(handle, placement);
        handle
    }

    /// Add a relative (child) placement and return its handle.
    ///
    /// Returns `None` if the parent handle does not exist.
    pub fn add_child(&mut self, child: RelativePlacement) -> Option<u64> {
        let parent = child.parent_handle;
        if !self.placements.contains_key(&parent) {
            return None;
        }

        let handle = self.next_handle;
        self.next_handle += 1;
        self.children.insert(handle, child);
        self.parent_children
            .entry(parent)
            .or_default()
            .insert(handle);
        Some(handle)
    }

    /// Remove a virtual placement by handle.
    ///
    /// This performs cascade deletion: all children of this placement are also removed.
    /// Returns the removed placement if it existed.
    pub fn remove(&mut self, handle: u64) -> Option<VirtualPlacement> {
        let placement = self.placements.remove(&handle)?;

        // Cascade: remove all children
        if let Some(child_handles) = self.parent_children.remove(&handle) {
            for child_handle in child_handles {
                self.children.remove(&child_handle);
            }
        }

        Some(placement)
    }

    /// Remove a child placement by handle.
    ///
    /// Returns the removed child if it existed.
    pub fn remove_child(&mut self, handle: u64) -> Option<RelativePlacement> {
        let child = self.children.remove(&handle)?;
        if let Some(siblings) = self.parent_children.get_mut(&child.parent_handle) {
            siblings.remove(&handle);
            if siblings.is_empty() {
                self.parent_children.remove(&child.parent_handle);
            }
        }
        Some(child)
    }

    /// Get a reference to a virtual placement.
    #[must_use]
    pub fn get(&self, handle: u64) -> Option<&VirtualPlacement> {
        self.placements.get(&handle)
    }

    /// Get a mutable reference to a virtual placement.
    pub fn get_mut(&mut self, handle: u64) -> Option<&mut VirtualPlacement> {
        self.placements.get_mut(&handle)
    }

    /// Get a reference to a child placement.
    #[must_use]
    pub fn get_child(&self, handle: u64) -> Option<&RelativePlacement> {
        self.children.get(&handle)
    }

    /// Get handles of all children of a given parent.
    #[must_use]
    pub fn children_of(&self, parent_handle: u64) -> Vec<u64> {
        self.parent_children
            .get(&parent_handle)
            .map_or_else(Vec::new, |set| set.iter().copied().collect())
    }

    /// Return the number of virtual placements (not counting children).
    #[must_use]
    pub fn len(&self) -> usize {
        self.placements.len()
    }

    /// Return the total number of child placements.
    #[must_use]
    pub fn children_count(&self) -> usize {
        self.children.len()
    }

    /// Return true if there are no placements.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.placements.is_empty()
    }

    /// Clear all placements and children.
    pub fn clear(&mut self) {
        self.placements.clear();
        self.children.clear();
        self.parent_children.clear();
    }

    /// Generate Kitty delete commands for a placement and all its children.
    ///
    /// Uses `d=i` (delete by image ID) for each unique image ID involved.
    #[must_use]
    pub fn encode_delete(&self, handle: u64) -> Vec<u8> {
        let mut buf = Vec::new();
        let mut deleted_ids = HashSet::new();

        if let Some(placement) = self.placements.get(&handle) {
            let img_id = placement.placement.image_id;
            if deleted_ids.insert(img_id) {
                match placement.placement.placement_id {
                    Some(pid) => {
                        buf.extend_from_slice(
                            format!("\x1b_Ga=d,d=i,i={img_id},p={pid};\x1b\\").as_bytes(),
                        );
                    }
                    None => {
                        buf.extend_from_slice(
                            format!("\x1b_Ga=d,d=i,i={img_id};\x1b\\").as_bytes(),
                        );
                    }
                }
            }

            // Delete children too
            if let Some(child_handles) = self.parent_children.get(&handle) {
                for &ch in child_handles {
                    if let Some(child) = self.children.get(&ch) {
                        let child_img_id = child.placement.image_id;
                        if deleted_ids.insert(child_img_id) {
                            match child.placement.placement_id {
                                Some(pid) => {
                                    buf.extend_from_slice(
                                        format!("\x1b_Ga=d,d=i,i={child_img_id},p={pid};\x1b\\")
                                            .as_bytes(),
                                    );
                                }
                                None => {
                                    buf.extend_from_slice(
                                        format!("\x1b_Ga=d,d=i,i={child_img_id};\x1b\\").as_bytes(),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        buf
    }

    /// Encode all virtual placements and their placeholder text.
    ///
    /// Returns a byte buffer containing the escape sequences and placeholder
    /// characters for all managed virtual placements, sorted by z-index.
    #[must_use]
    pub fn encode_all(&self) -> Vec<u8> {
        let mut entries: Vec<(u64, &VirtualPlacement)> =
            self.placements.iter().map(|(&h, p)| (h, p)).collect();
        entries.sort_by(|a, b| {
            a.1.placement
                .z_index
                .cmp(&b.1.placement.z_index)
                .then(a.0.cmp(&b.0))
        });

        let mut buf = Vec::new();
        for (handle, vp) in entries {
            buf.extend_from_slice(&vp.encode_full());

            // Encode children of this placement
            if let Some(child_handles) = self.parent_children.get(&handle) {
                let mut child_entries: Vec<_> = child_handles
                    .iter()
                    .filter_map(|&ch| self.children.get(&ch).map(|c| (ch, c)))
                    .collect();
                child_entries.sort_by_key(|(h, _)| *h);

                for (_, child) in child_entries {
                    if let Some(parent_pid) = vp.placement.placement_id {
                        buf.extend_from_slice(&child.encode_create(parent_pid));
                    }
                }
            }
        }

        buf
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Encoding helpers --

    #[test]
    fn encode_diacritic_range() {
        let d0 = encode_diacritic(0);
        assert_eq!(d0 as u32, DIACRITICS_START);

        let d255 = encode_diacritic(255);
        assert_eq!(d255 as u32, DIACRITICS_START + 255);
    }

    #[test]
    fn encode_decode_id_roundtrip() {
        for id in [0u32, 1, 42, 255, 256, 1000, 65535, 16_777_215, u32::MAX] {
            let diacritics = encode_id_diacritics(id);
            let decoded = decode_id_diacritics(&diacritics);
            assert_eq!(decoded, id, "roundtrip failed for id={id}");
        }
    }

    #[test]
    fn encode_id_compact_representation() {
        // Small ID should use fewer diacritics
        let d1 = encode_id_diacritics(1);
        assert_eq!(d1.len(), 1);

        let d256 = encode_id_diacritics(256);
        assert_eq!(d256.len(), 2);

        let d_big = encode_id_diacritics(u32::MAX);
        assert_eq!(d_big.len(), 4);
    }

    #[test]
    fn encode_row_col_values() {
        let (r, c) = encode_row_col(0, 0);
        assert_eq!(r as u32, DIACRITICS_START);
        assert_eq!(c as u32, DIACRITICS_START);

        let (r, c) = encode_row_col(5, 10);
        assert_eq!(r as u32, DIACRITICS_START + 5);
        assert_eq!(c as u32, DIACRITICS_START + 10);
    }

    // -- Placeholder generation --

    #[test]
    fn placeholder_single_cell() {
        let text = generate_placeholder(1, None, 1, 1);
        let chars: Vec<char> = text.chars().collect();

        // Should start with placeholder char
        assert_eq!(chars[0], PLACEHOLDER_CHAR);
        // Should have row diacritic, col diacritic, and at least 1 ID diacritic
        assert!(chars.len() >= 4); // placeholder + row + col + id
    }

    #[test]
    fn placeholder_multiline() {
        let text = generate_placeholder(1, None, 3, 2);
        let lines: Vec<&str> = text.split('\n').collect();
        assert_eq!(lines.len(), 3);

        // Each line should have 2 placeholder groups
        for line in &lines {
            let placeholder_count = line.chars().filter(|&c| c == PLACEHOLDER_CHAR).count();
            assert_eq!(placeholder_count, 2);
        }
    }

    #[test]
    fn placeholder_with_placement_id() {
        let without_pid = generate_placeholder(1, None, 1, 1);
        let with_pid = generate_placeholder(1, Some(5), 1, 1);

        // With placement ID should be longer (extra diacritics)
        assert!(with_pid.len() > without_pid.len());
    }

    #[test]
    fn placeholder_contains_correct_char() {
        let text = generate_placeholder(42, None, 2, 3);
        for ch in text.chars() {
            if ch == '\n' {
                continue;
            }
            // Should be either placeholder char or a combining diacritic
            assert!(
                ch == PLACEHOLDER_CHAR || (ch as u32 >= DIACRITICS_START),
                "unexpected character: U+{:04X}",
                ch as u32
            );
        }
    }

    #[test]
    fn is_placeholder_char_works() {
        assert!(is_placeholder_char('\u{10EEEE}'));
        assert!(!is_placeholder_char('A'));
        assert!(!is_placeholder_char('\u{0305}'));
    }

    // -- VirtualPlacement --

    #[test]
    fn virtual_placement_creation() {
        let vp = VirtualPlacement::new(42, 5, 10);
        assert_eq!(vp.placement.image_id, 42);
        assert_eq!(vp.rows, 5);
        assert_eq!(vp.cols, 10);
    }

    #[test]
    fn virtual_placement_min_size() {
        let vp = VirtualPlacement::new(1, 0, 0);
        assert_eq!(vp.rows, 1);
        assert_eq!(vp.cols, 1);
    }

    #[test]
    fn virtual_placement_builder_chain() {
        let vp = VirtualPlacement::new(1, 3, 4)
            .with_placement_id(7)
            .scale_mode(ScaleMode::Cover)
            .on_layer(ImageLayer::BelowText)
            .with_z_index(5);

        assert_eq!(vp.placement.placement_id, Some(7));
        assert_eq!(vp.placement.scale, ScaleMode::Cover);
        assert_eq!(vp.placement.layer, ImageLayer::BelowText);
        assert_eq!(vp.placement.z_index, 5);
    }

    #[test]
    fn virtual_placement_encode_create() {
        let vp = VirtualPlacement::new(42, 3, 5).with_placement_id(1);
        let bytes = vp.encode_create();
        let text = String::from_utf8(bytes).unwrap();

        assert!(text.starts_with("\x1b_G"));
        assert!(text.ends_with("\x1b\\"));
        assert!(text.contains("a=p"));
        assert!(text.contains("i=42"));
        assert!(text.contains("p=1"));
        assert!(text.contains("U=1"));
        assert!(text.contains("c=5"));
        assert!(text.contains("r=3"));
    }

    #[test]
    fn virtual_placement_encode_no_z_index_if_zero() {
        let vp = VirtualPlacement::new(1, 2, 2);
        let bytes = vp.encode_create();
        let text = String::from_utf8(bytes).unwrap();
        assert!(!text.contains("z="));
    }

    #[test]
    fn virtual_placement_encode_z_index() {
        let vp = VirtualPlacement::new(1, 2, 2).with_z_index(-3);
        let bytes = vp.encode_create();
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("z=-3"));
    }

    #[test]
    fn virtual_placement_encode_cursor_suppress() {
        let mut vp = VirtualPlacement::new(1, 2, 2);
        vp.placement = vp.placement.cursor_movement(CursorMovement::Suppress);
        let bytes = vp.encode_create();
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("C=1"));
    }

    #[test]
    fn virtual_placement_generate_placeholder() {
        let vp = VirtualPlacement::new(42, 2, 3);
        let placeholder = vp.generate_placeholder();
        let lines: Vec<&str> = placeholder.split('\n').collect();
        assert_eq!(lines.len(), 2);
        for line in &lines {
            assert_eq!(line.chars().filter(|&c| c == PLACEHOLDER_CHAR).count(), 3);
        }
    }

    #[test]
    fn virtual_placement_encode_full() {
        let vp = VirtualPlacement::new(42, 1, 1).with_placement_id(1);
        let bytes = vp.encode_full();
        let text = String::from_utf8_lossy(&bytes);

        // Should start with escape sequence
        assert!(text.starts_with("\x1b_G"));
        // Should contain placeholder character
        assert!(text.contains('\u{10EEEE}'));
    }

    // -- RelativePlacement --

    #[test]
    fn relative_placement_creation() {
        let rp = RelativePlacement::new(10, 0, 2, 3);
        assert_eq!(rp.placement.image_id, 10);
        assert_eq!(rp.parent_handle, 0);
        assert_eq!(rp.offset_col, 0);
        assert_eq!(rp.offset_row, 0);
        assert_eq!(rp.rows, 2);
        assert_eq!(rp.cols, 3);
    }

    #[test]
    fn relative_placement_offset() {
        let rp = RelativePlacement::new(1, 0, 1, 1).offset(5, -2);
        assert_eq!(rp.offset_col, 5);
        assert_eq!(rp.offset_row, -2);
    }

    #[test]
    fn relative_placement_absolute_position() {
        let rp = RelativePlacement::new(1, 0, 1, 1).offset(3, 2);
        let (col, row) = rp.absolute_position(10, 5);
        assert_eq!(col, 13);
        assert_eq!(row, 7);
    }

    #[test]
    fn relative_placement_absolute_position_clamps_negative() {
        let rp = RelativePlacement::new(1, 0, 1, 1).offset(-20, -20);
        let (col, row) = rp.absolute_position(5, 5);
        assert_eq!(col, 0);
        assert_eq!(row, 0);
    }

    #[test]
    fn relative_placement_encode_create() {
        let rp = RelativePlacement::new(10, 0, 2, 3)
            .offset(5, -1)
            .with_placement_id(99);
        let bytes = rp.encode_create(42);
        let text = String::from_utf8(bytes).unwrap();

        assert!(text.starts_with("\x1b_G"));
        assert!(text.ends_with("\x1b\\"));
        assert!(text.contains("a=p"));
        assert!(text.contains("i=10"));
        assert!(text.contains("p=99"));
        assert!(text.contains("U=1"));
        assert!(text.contains("P=42")); // parent placement ID
        assert!(text.contains("H=5")); // horizontal offset
        assert!(text.contains("V=-1")); // vertical offset
        assert!(text.contains("c=3"));
        assert!(text.contains("r=2"));
    }

    #[test]
    fn relative_placement_encode_no_offset_if_zero() {
        let rp = RelativePlacement::new(1, 0, 1, 1);
        let bytes = rp.encode_create(1);
        let text = String::from_utf8(bytes).unwrap();
        assert!(!text.contains("H="));
        assert!(!text.contains("V="));
    }

    // -- VirtualPlacementManager --

    #[test]
    fn manager_add_and_get() {
        let mut mgr = VirtualPlacementManager::new();
        assert!(mgr.is_empty());

        let h = mgr.add(VirtualPlacement::new(1, 3, 4));
        assert_eq!(mgr.len(), 1);

        let vp = mgr.get(h).unwrap();
        assert_eq!(vp.placement.image_id, 1);
        assert_eq!(vp.rows, 3);
        assert_eq!(vp.cols, 4);
    }

    #[test]
    fn manager_remove() {
        let mut mgr = VirtualPlacementManager::new();
        let h = mgr.add(VirtualPlacement::new(1, 2, 2));
        let removed = mgr.remove(h);
        assert!(removed.is_some());
        assert!(mgr.is_empty());
        assert!(mgr.remove(h).is_none());
    }

    #[test]
    fn manager_get_mut() {
        let mut mgr = VirtualPlacementManager::new();
        let h = mgr.add(VirtualPlacement::new(1, 2, 2));
        if let Some(vp) = mgr.get_mut(h) {
            vp.rows = 10;
        }
        assert_eq!(mgr.get(h).unwrap().rows, 10);
    }

    #[test]
    fn manager_add_child() {
        let mut mgr = VirtualPlacementManager::new();
        let parent = mgr.add(VirtualPlacement::new(1, 5, 5).with_placement_id(10));
        let child_handle = mgr
            .add_child(RelativePlacement::new(2, parent, 2, 2).offset(1, 1))
            .unwrap();

        assert_eq!(mgr.children_count(), 1);
        let child = mgr.get_child(child_handle).unwrap();
        assert_eq!(child.placement.image_id, 2);
        assert_eq!(child.offset_col, 1);
    }

    #[test]
    fn manager_add_child_invalid_parent() {
        let mut mgr = VirtualPlacementManager::new();
        let result = mgr.add_child(RelativePlacement::new(2, 999, 1, 1));
        assert!(result.is_none());
    }

    #[test]
    fn manager_cascade_delete() {
        let mut mgr = VirtualPlacementManager::new();
        let parent = mgr.add(VirtualPlacement::new(1, 5, 5).with_placement_id(10));
        let _child1 = mgr
            .add_child(RelativePlacement::new(2, parent, 2, 2))
            .unwrap();
        let _child2 = mgr
            .add_child(RelativePlacement::new(3, parent, 1, 1))
            .unwrap();

        assert_eq!(mgr.children_count(), 2);

        // Removing parent should cascade delete children
        mgr.remove(parent);
        assert!(mgr.is_empty());
        assert_eq!(mgr.children_count(), 0);
    }

    #[test]
    fn manager_remove_single_child() {
        let mut mgr = VirtualPlacementManager::new();
        let parent = mgr.add(VirtualPlacement::new(1, 5, 5));
        let child1 = mgr
            .add_child(RelativePlacement::new(2, parent, 2, 2))
            .unwrap();
        let child2 = mgr
            .add_child(RelativePlacement::new(3, parent, 1, 1))
            .unwrap();

        mgr.remove_child(child1);
        assert_eq!(mgr.children_count(), 1);
        assert!(mgr.get_child(child1).is_none());
        assert!(mgr.get_child(child2).is_some());

        // Children list should be updated
        let children = mgr.children_of(parent);
        assert_eq!(children.len(), 1);
        assert!(children.contains(&child2));
    }

    #[test]
    fn manager_children_of() {
        let mut mgr = VirtualPlacementManager::new();
        let parent = mgr.add(VirtualPlacement::new(1, 5, 5));
        let c1 = mgr
            .add_child(RelativePlacement::new(2, parent, 1, 1))
            .unwrap();
        let c2 = mgr
            .add_child(RelativePlacement::new(3, parent, 1, 1))
            .unwrap();

        let children = mgr.children_of(parent);
        assert_eq!(children.len(), 2);
        assert!(children.contains(&c1));
        assert!(children.contains(&c2));

        // Non-existent parent
        assert!(mgr.children_of(999).is_empty());
    }

    #[test]
    fn manager_clear() {
        let mut mgr = VirtualPlacementManager::new();
        let parent = mgr.add(VirtualPlacement::new(1, 3, 3));
        mgr.add_child(RelativePlacement::new(2, parent, 1, 1));

        mgr.clear();
        assert!(mgr.is_empty());
        assert_eq!(mgr.children_count(), 0);
    }

    #[test]
    fn manager_encode_delete() {
        let mut mgr = VirtualPlacementManager::new();
        let parent = mgr.add(VirtualPlacement::new(1, 3, 3).with_placement_id(10));
        mgr.add_child(RelativePlacement::new(2, parent, 1, 1).with_placement_id(20));

        let bytes = mgr.encode_delete(parent);
        let text = String::from_utf8(bytes).unwrap();

        // Should contain delete commands for both parent and child image IDs
        assert!(text.contains("a=d"));
        assert!(text.contains("i=1"));
        assert!(text.contains("i=2"));
    }

    #[test]
    fn manager_encode_delete_nonexistent() {
        let mgr = VirtualPlacementManager::new();
        let bytes = mgr.encode_delete(999);
        assert!(bytes.is_empty());
    }

    #[test]
    fn manager_encode_all() {
        let mut mgr = VirtualPlacementManager::new();
        let parent = mgr.add(VirtualPlacement::new(1, 2, 2).with_placement_id(10));
        mgr.add_child(RelativePlacement::new(2, parent, 1, 1).with_placement_id(20));

        let bytes = mgr.encode_all();
        let text = String::from_utf8_lossy(&bytes);

        // Should contain parent virtual placement
        assert!(text.contains("U=1"));
        assert!(text.contains("i=1"));
        // Should contain placeholder characters
        assert!(text.contains('\u{10EEEE}'));
        // Should contain child placement referencing parent
        assert!(text.contains("P=10"));
        assert!(text.contains("i=2"));
    }

    #[test]
    fn manager_encode_all_sorted_by_z_index() {
        let mut mgr = VirtualPlacementManager::new();
        mgr.add(VirtualPlacement::new(1, 1, 1).with_z_index(10));
        mgr.add(VirtualPlacement::new(2, 1, 1).with_z_index(-5));

        let bytes = mgr.encode_all();
        let text = String::from_utf8_lossy(&bytes);

        // Image 2 (z=-5) should appear before image 1 (z=10)
        let pos_i2 = text.find("i=2").unwrap();
        let pos_i1 = text.find("i=1").unwrap();
        assert!(pos_i2 < pos_i1, "z=-5 should be encoded before z=10");
    }

    #[test]
    fn manager_multiple_parents_independent() {
        let mut mgr = VirtualPlacementManager::new();
        let p1 = mgr.add(VirtualPlacement::new(1, 3, 3));
        let p2 = mgr.add(VirtualPlacement::new(2, 3, 3));

        let c1 = mgr.add_child(RelativePlacement::new(10, p1, 1, 1)).unwrap();
        let c2 = mgr.add_child(RelativePlacement::new(20, p2, 1, 1)).unwrap();

        // Removing p1 should not affect p2 or its children
        mgr.remove(p1);
        assert_eq!(mgr.len(), 1);
        assert!(mgr.get_child(c1).is_none());
        assert!(mgr.get_child(c2).is_some());
    }

    #[test]
    fn manager_remove_all_children_cleans_parent_map() {
        let mut mgr = VirtualPlacementManager::new();
        let parent = mgr.add(VirtualPlacement::new(1, 3, 3));
        let c1 = mgr
            .add_child(RelativePlacement::new(2, parent, 1, 1))
            .unwrap();
        let c2 = mgr
            .add_child(RelativePlacement::new(3, parent, 1, 1))
            .unwrap();

        mgr.remove_child(c1);
        mgr.remove_child(c2);

        // Parent should have no children entry
        assert!(mgr.children_of(parent).is_empty());
    }

    // -- Edge cases --

    #[test]
    fn placeholder_large_grid() {
        let text = generate_placeholder(1, None, 255, 255);
        let lines: Vec<&str> = text.split('\n').collect();
        assert_eq!(lines.len(), 255);
        for line in &lines {
            assert_eq!(line.chars().filter(|&c| c == PLACEHOLDER_CHAR).count(), 255);
        }
    }

    #[test]
    fn encode_id_zero() {
        let d = encode_id_diacritics(0);
        assert_eq!(d.len(), 1);
        assert_eq!(decode_id_diacritics(&d), 0);
    }

    #[test]
    fn relative_placement_min_size() {
        let rp = RelativePlacement::new(1, 0, 0, 0);
        assert_eq!(rp.rows, 1);
        assert_eq!(rp.cols, 1);
    }
}
