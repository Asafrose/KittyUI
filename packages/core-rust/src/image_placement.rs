//! Image placement, scaling, and layout integration for the Kitty graphics protocol.
//!
//! This module builds on top of `image.rs` (encoding/transmission) to provide:
//! - Cell-based positioning and sizing
//! - Scaling modes (contain, cover, fill, none)
//! - Source rectangle cropping (sub-region display)
//! - Z-index layering (above or below text)
//! - Multiple placements of the same image
//! - Cursor movement control
//! - Integration with the Taffy layout engine

use std::collections::HashMap;

use crate::layout::ComputedLayout;

// ---------------------------------------------------------------------------
// Scaling
// ---------------------------------------------------------------------------

/// How an image is scaled to fit its placement area.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ScaleMode {
    /// Scale uniformly so the entire image fits, preserving aspect ratio.
    /// The image may be letter-boxed.
    #[default]
    Contain,
    /// Scale uniformly so the image covers the entire area, preserving
    /// aspect ratio. Parts of the image may be clipped.
    Cover,
    /// Stretch the image to exactly fill the area, ignoring aspect ratio.
    Fill,
    /// Display the image at its native pixel size (no scaling).
    None,
}

// ---------------------------------------------------------------------------
// Source rectangle (cropping)
// ---------------------------------------------------------------------------

/// A rectangle within the source image, in pixels.
///
/// Used to display a sub-region of an image.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SourceRect {
    /// X offset in pixels from the left edge of the source image.
    pub x: u32,
    /// Y offset in pixels from the top edge of the source image.
    pub y: u32,
    /// Width in pixels. `0` means "rest of the image".
    pub width: u32,
    /// Height in pixels. `0` means "rest of the image".
    pub height: u32,
}

impl SourceRect {
    /// Create a new source rectangle.
    #[must_use]
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Resolve zero-means-rest dimensions against the full image size.
    #[must_use]
    pub fn resolve(&self, image_width: u32, image_height: u32) -> Self {
        let w = if self.width == 0 {
            image_width.saturating_sub(self.x)
        } else {
            self.width
        };
        let h = if self.height == 0 {
            image_height.saturating_sub(self.y)
        } else {
            self.height
        };
        Self {
            x: self.x,
            y: self.y,
            width: w,
            height: h,
        }
    }
}

// ---------------------------------------------------------------------------
// Z-index / layer
// ---------------------------------------------------------------------------

/// Whether an image is rendered above or below text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ImageLayer {
    /// Render below text (background).
    BelowText,
    /// Render above text (foreground, the default in Kitty).
    #[default]
    AboveText,
}

// ---------------------------------------------------------------------------
// Cursor movement
// ---------------------------------------------------------------------------

/// Controls whether the terminal cursor advances after placing an image.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CursorMovement {
    /// The cursor moves past the image (default terminal behaviour).
    #[default]
    Advance,
    /// The cursor stays in place (the image is placed without moving the cursor).
    Suppress,
}

// ---------------------------------------------------------------------------
// Placement
// ---------------------------------------------------------------------------

/// A single placement of an image on screen.
///
/// Multiple placements can reference the same `image_id` (the image data
/// transmitted via the Kitty protocol). Each placement has its own position,
/// size, scaling, cropping, z-index, and cursor behaviour.
#[derive(Clone, Debug, PartialEq)]
pub struct ImagePlacement {
    /// ID of the transmitted image (from `image::ImageCache`).
    pub image_id: u32,
    /// Optional placement ID (for targeting specific placements in updates/deletes).
    pub placement_id: Option<u32>,
    /// Column position (cell coordinates).
    pub col: u32,
    /// Row position (cell coordinates).
    pub row: u32,
    /// Display width in cells. `None` means use image native size / scaling.
    pub width_cells: Option<u32>,
    /// Display height in cells. `None` means use image native size / scaling.
    pub height_cells: Option<u32>,
    /// Scaling mode.
    pub scale: ScaleMode,
    /// Optional source rectangle for cropping.
    pub source_rect: Option<SourceRect>,
    /// Z-index layer.
    pub layer: ImageLayer,
    /// Cursor movement behaviour.
    pub cursor: CursorMovement,
    /// Z-index value for ordering among images on the same layer.
    /// Higher values are rendered on top.
    pub z_index: i32,
}

impl ImagePlacement {
    /// Create a new placement with default settings.
    #[must_use]
    pub fn new(image_id: u32) -> Self {
        Self {
            image_id,
            placement_id: None,
            col: 0,
            row: 0,
            width_cells: None,
            height_cells: None,
            scale: ScaleMode::default(),
            source_rect: None,
            layer: ImageLayer::default(),
            cursor: CursorMovement::default(),
            z_index: 0,
        }
    }

    /// Set the cell position.
    #[must_use]
    pub fn at(mut self, col: u32, row: u32) -> Self {
        self.col = col;
        self.row = row;
        self
    }

    /// Set the display size in cells.
    #[must_use]
    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.width_cells = Some(width);
        self.height_cells = Some(height);
        self
    }

    /// Set the scaling mode.
    #[must_use]
    pub fn scale_mode(mut self, mode: ScaleMode) -> Self {
        self.scale = mode;
        self
    }

    /// Set a source rectangle for cropping.
    #[must_use]
    pub fn crop(mut self, rect: SourceRect) -> Self {
        self.source_rect = Some(rect);
        self
    }

    /// Set the z-index layer.
    #[must_use]
    pub fn on_layer(mut self, layer: ImageLayer) -> Self {
        self.layer = layer;
        self
    }

    /// Set cursor movement behaviour.
    #[must_use]
    pub fn cursor_movement(mut self, movement: CursorMovement) -> Self {
        self.cursor = movement;
        self
    }

    /// Set the z-index value.
    #[must_use]
    pub fn with_z_index(mut self, z: i32) -> Self {
        self.z_index = z;
        self
    }

    /// Set the placement ID.
    #[must_use]
    pub fn with_placement_id(mut self, id: u32) -> Self {
        self.placement_id = Some(id);
        self
    }
}

// ---------------------------------------------------------------------------
// Computed scaling
// ---------------------------------------------------------------------------

/// Result of computing how an image should be scaled and positioned.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScaledRegion {
    /// Source X offset in pixels.
    pub src_x: u32,
    /// Source Y offset in pixels.
    pub src_y: u32,
    /// Source width in pixels.
    pub src_w: u32,
    /// Source height in pixels.
    pub src_h: u32,
    /// Display width in cells.
    pub dst_cols: u32,
    /// Display height in cells.
    pub dst_rows: u32,
    /// Column offset within the placement area (for letter-boxing).
    pub offset_col: u32,
    /// Row offset within the placement area (for letter-boxing).
    pub offset_row: u32,
}

/// Compute the scaled region for a placement.
///
/// `image_width` and `image_height` are the full source image dimensions in pixels.
/// `cell_width_px` and `cell_height_px` are the terminal cell dimensions in pixels.
#[must_use]
#[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
pub fn compute_scaling(
    placement: &ImagePlacement,
    image_width: u32,
    image_height: u32,
    cell_width_px: f32,
    cell_height_px: f32,
) -> ScaledRegion {
    // Resolve source rectangle.
    let src = placement
        .source_rect
        .map_or(SourceRect::new(0, 0, image_width, image_height), |r| {
            r.resolve(image_width, image_height)
        });

    let src_w = src.width.max(1);
    let src_h = src.height.max(1);

    // Target area in cells (default: compute from image native size).
    let target_cols = placement.width_cells.unwrap_or_else(|| {
        let px = src_w as f32;
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let cells = (px / cell_width_px).ceil() as u32;
        cells.max(1)
    });
    let target_rows = placement.height_cells.unwrap_or_else(|| {
        let px = src_h as f32;
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let cells = (px / cell_height_px).ceil() as u32;
        cells.max(1)
    });

    match placement.scale {
        ScaleMode::Fill => ScaledRegion {
            src_x: src.x,
            src_y: src.y,
            src_w,
            src_h,
            dst_cols: target_cols,
            dst_rows: target_rows,
            offset_col: 0,
            offset_row: 0,
        },
        ScaleMode::None => {
            // Display at native size, clipped to target area.
            let native_cols = {
                let px = src_w as f32;
                #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                let cells = (px / cell_width_px).ceil() as u32;
                cells
            };
            let native_rows = {
                let px = src_h as f32;
                #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                let cells = (px / cell_height_px).ceil() as u32;
                cells
            };
            ScaledRegion {
                src_x: src.x,
                src_y: src.y,
                src_w,
                src_h,
                dst_cols: native_cols.min(target_cols).max(1),
                dst_rows: native_rows.min(target_rows).max(1),
                offset_col: 0,
                offset_row: 0,
            }
        }
        ScaleMode::Contain => {
            // Fit within target, preserving aspect ratio.
            let src_aspect = src_w as f32 / src_h as f32;
            let target_width_px = target_cols as f32 * cell_width_px;
            let target_height_px = target_rows as f32 * cell_height_px;
            let target_aspect = target_width_px / target_height_px;

            let (fit_cols, fit_rows) = if src_aspect > target_aspect {
                // Image is wider than target: fit to width.
                let fit_h_px = target_width_px / src_aspect;
                #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                let rows = (fit_h_px / cell_height_px).ceil() as u32;
                (target_cols, rows.min(target_rows).max(1))
            } else {
                // Image is taller than target: fit to height.
                let fit_w_px = target_height_px * src_aspect;
                #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                let cols = (fit_w_px / cell_width_px).ceil() as u32;
                (cols.min(target_cols).max(1), target_rows)
            };

            // Center within the target area.
            let offset_col = (target_cols.saturating_sub(fit_cols)) / 2;
            let offset_row = (target_rows.saturating_sub(fit_rows)) / 2;

            ScaledRegion {
                src_x: src.x,
                src_y: src.y,
                src_w,
                src_h,
                dst_cols: fit_cols,
                dst_rows: fit_rows,
                offset_col,
                offset_row,
            }
        }
        ScaleMode::Cover => {
            // Cover the entire target, preserving aspect ratio (may crop).
            let src_aspect = src_w as f32 / src_h as f32;
            let target_width_px = target_cols as f32 * cell_width_px;
            let target_height_px = target_rows as f32 * cell_height_px;
            let target_aspect = target_width_px / target_height_px;

            // Compute the visible portion of the source.
            let (visible_w, visible_h) = if src_aspect > target_aspect {
                // Image is wider: crop horizontally.
                #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                let w = (src_h as f32 * target_aspect) as u32;
                (w.min(src_w).max(1), src_h)
            } else {
                // Image is taller: crop vertically.
                #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                let h = (src_w as f32 / target_aspect) as u32;
                (src_w, h.min(src_h).max(1))
            };

            // Center the crop within the source.
            let crop_x = src.x + (src_w.saturating_sub(visible_w)) / 2;
            let crop_y = src.y + (src_h.saturating_sub(visible_h)) / 2;

            ScaledRegion {
                src_x: crop_x,
                src_y: crop_y,
                src_w: visible_w,
                src_h: visible_h,
                dst_cols: target_cols,
                dst_rows: target_rows,
                offset_col: 0,
                offset_row: 0,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Kitty protocol encoding for placements
// ---------------------------------------------------------------------------

/// Encode a placement command using the Kitty graphics protocol.
///
/// This generates the escape sequence to display an already-transmitted image
/// at a specific position with the given scaling and options.
#[must_use]
pub fn encode_placement(placement: &ImagePlacement, scaled: &ScaledRegion) -> Vec<u8> {
    let mut params = Vec::new();

    // Action: display (place).
    params.push("a=p".to_owned());
    // Image ID.
    params.push(format!("i={}", placement.image_id));

    // Placement ID.
    if let Some(pid) = placement.placement_id {
        params.push(format!("p={pid}"));
    }

    // Source rectangle (cropping) — only if not the full image.
    if scaled.src_x > 0 {
        params.push(format!("x={}", scaled.src_x));
    }
    if scaled.src_y > 0 {
        params.push(format!("y={}", scaled.src_y));
    }
    if placement.source_rect.is_some() {
        params.push(format!("w={}", scaled.src_w));
        params.push(format!("h={}", scaled.src_h));
    }

    // Display size in cells.
    if placement.width_cells.is_some() || placement.scale != ScaleMode::None {
        params.push(format!("c={}", scaled.dst_cols));
    }
    if placement.height_cells.is_some() || placement.scale != ScaleMode::None {
        params.push(format!("r={}", scaled.dst_rows));
    }

    // Z-index.
    if placement.z_index != 0 {
        params.push(format!("z={}", placement.z_index));
    }

    // Cursor movement: C=1 means do not move the cursor.
    if placement.cursor == CursorMovement::Suppress {
        params.push("C=1".to_owned());
    }

    let param_str = params.join(",");
    format!("\x1b_G{param_str};\x1b\\").into_bytes()
}

/// Encode the cursor-positioning prefix to move to (col, row) before placing.
///
/// Uses the standard ANSI CUP (Cursor Position) escape: `ESC[row;colH`.
/// Kitty uses 1-based coordinates.
#[must_use]
pub fn encode_cursor_move(col: u32, row: u32) -> Vec<u8> {
    // CUP is 1-based: ESC [ Pr ; Pc H
    format!("\x1b[{};{}H", row + 1, col + 1).into_bytes()
}

/// Encode a full placement sequence: move cursor, then place image.
#[must_use]
pub fn encode_full_placement(placement: &ImagePlacement, scaled: &ScaledRegion) -> Vec<u8> {
    let effective_col = placement.col + scaled.offset_col;
    let effective_row = placement.row + scaled.offset_row;

    let mut buf = encode_cursor_move(effective_col, effective_row);
    buf.extend_from_slice(&encode_placement(placement, scaled));
    buf
}

// ---------------------------------------------------------------------------
// Placement manager
// ---------------------------------------------------------------------------

/// Manages multiple image placements, supporting add, remove, update,
/// and sorted rendering by z-index.
#[derive(Debug, Default)]
pub struct PlacementManager {
    /// All placements, keyed by a caller-provided handle.
    placements: HashMap<u64, ImagePlacement>,
    /// Source image dimensions for each `image_id`: `(width_px, height_px)`.
    image_dimensions: HashMap<u32, (u32, u32)>,
    /// Next auto-assigned handle.
    next_handle: u64,
}

impl PlacementManager {
    /// Create a new empty placement manager.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register the pixel dimensions for a transmitted image.
    pub fn register_image(&mut self, image_id: u32, width: u32, height: u32) {
        self.image_dimensions.insert(image_id, (width, height));
    }

    /// Add a placement and return its handle.
    pub fn add(&mut self, placement: ImagePlacement) -> u64 {
        let handle = self.next_handle;
        self.next_handle += 1;
        self.placements.insert(handle, placement);
        handle
    }

    /// Remove a placement by handle. Returns the placement if it existed.
    pub fn remove(&mut self, handle: u64) -> Option<ImagePlacement> {
        self.placements.remove(&handle)
    }

    /// Get a reference to a placement by handle.
    #[must_use]
    pub fn get(&self, handle: u64) -> Option<&ImagePlacement> {
        self.placements.get(&handle)
    }

    /// Get a mutable reference to a placement by handle.
    pub fn get_mut(&mut self, handle: u64) -> Option<&mut ImagePlacement> {
        self.placements.get_mut(&handle)
    }

    /// Return all placements sorted by z-index (lowest first) then by layer
    /// (below-text before above-text).
    #[must_use]
    pub fn sorted_placements(&self) -> Vec<(u64, &ImagePlacement)> {
        let mut entries: Vec<(u64, &ImagePlacement)> =
            self.placements.iter().map(|(&h, p)| (h, p)).collect();
        entries.sort_by(|a, b| {
            let layer_ord = |l: &ImageLayer| match l {
                ImageLayer::BelowText => 0,
                ImageLayer::AboveText => 1,
            };
            layer_ord(&a.1.layer)
                .cmp(&layer_ord(&b.1.layer))
                .then(a.1.z_index.cmp(&b.1.z_index))
                .then(a.0.cmp(&b.0))
        });
        entries
    }

    /// Render all placements to a byte buffer, sorted by z-index.
    ///
    /// `cell_width_px` and `cell_height_px` are the terminal cell dimensions.
    #[must_use]
    pub fn render_all(&self, cell_width_px: f32, cell_height_px: f32) -> Vec<u8> {
        let mut buf = Vec::new();
        for (_, placement) in self.sorted_placements() {
            let (img_w, img_h) = self
                .image_dimensions
                .get(&placement.image_id)
                .copied()
                .unwrap_or((1, 1));
            let scaled = compute_scaling(placement, img_w, img_h, cell_width_px, cell_height_px);
            buf.extend_from_slice(&encode_full_placement(placement, &scaled));
        }
        buf
    }

    /// Return the number of placements.
    #[must_use]
    pub fn len(&self) -> usize {
        self.placements.len()
    }

    /// Return true if there are no placements.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.placements.is_empty()
    }

    /// Clear all placements.
    pub fn clear(&mut self) {
        self.placements.clear();
    }

    /// Get the pixel dimensions for a registered image.
    #[must_use]
    pub fn image_dimensions(&self, image_id: u32) -> Option<(u32, u32)> {
        self.image_dimensions.get(&image_id).copied()
    }
}

// ---------------------------------------------------------------------------
// Layout integration
// ---------------------------------------------------------------------------

/// Place an image within a computed layout node's bounds.
///
/// Converts the layout node's position and size (in cell coordinates) into
/// an `ImagePlacement` configured to fill that node's area.
#[must_use]
pub fn place_in_layout(
    image_id: u32,
    layout: &ComputedLayout,
    scale: ScaleMode,
    layer: ImageLayer,
) -> ImagePlacement {
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let col = layout.x.round() as u32;
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let row = layout.y.round() as u32;
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let w = layout.width.round() as u32;
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let h = layout.height.round() as u32;

    ImagePlacement::new(image_id)
        .at(col, row)
        .size(w.max(1), h.max(1))
        .scale_mode(scale)
        .on_layer(layer)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- SourceRect --

    #[test]
    fn source_rect_resolve_zero_means_rest() {
        let r = SourceRect::new(10, 20, 0, 0);
        let resolved = r.resolve(100, 80);
        assert_eq!(resolved.x, 10);
        assert_eq!(resolved.y, 20);
        assert_eq!(resolved.width, 90);
        assert_eq!(resolved.height, 60);
    }

    #[test]
    fn source_rect_resolve_explicit_dims() {
        let r = SourceRect::new(5, 5, 50, 40);
        let resolved = r.resolve(100, 80);
        assert_eq!(resolved.width, 50);
        assert_eq!(resolved.height, 40);
    }

    #[test]
    fn source_rect_resolve_offset_beyond_image() {
        let r = SourceRect::new(200, 200, 0, 0);
        let resolved = r.resolve(100, 80);
        assert_eq!(resolved.width, 0);
        assert_eq!(resolved.height, 0);
    }

    // -- ImagePlacement builder --

    #[test]
    fn placement_builder_chain() {
        let p = ImagePlacement::new(42)
            .at(5, 10)
            .size(20, 15)
            .scale_mode(ScaleMode::Cover)
            .crop(SourceRect::new(0, 0, 100, 100))
            .on_layer(ImageLayer::BelowText)
            .cursor_movement(CursorMovement::Suppress)
            .with_z_index(-1)
            .with_placement_id(7);

        assert_eq!(p.image_id, 42);
        assert_eq!(p.col, 5);
        assert_eq!(p.row, 10);
        assert_eq!(p.width_cells, Some(20));
        assert_eq!(p.height_cells, Some(15));
        assert_eq!(p.scale, ScaleMode::Cover);
        assert!(p.source_rect.is_some());
        assert_eq!(p.layer, ImageLayer::BelowText);
        assert_eq!(p.cursor, CursorMovement::Suppress);
        assert_eq!(p.z_index, -1);
        assert_eq!(p.placement_id, Some(7));
    }

    #[test]
    fn placement_defaults() {
        let p = ImagePlacement::new(1);
        assert_eq!(p.col, 0);
        assert_eq!(p.row, 0);
        assert_eq!(p.width_cells, None);
        assert_eq!(p.height_cells, None);
        assert_eq!(p.scale, ScaleMode::Contain);
        assert!(p.source_rect.is_none());
        assert_eq!(p.layer, ImageLayer::AboveText);
        assert_eq!(p.cursor, CursorMovement::Advance);
        assert_eq!(p.z_index, 0);
        assert_eq!(p.placement_id, None);
    }

    // -- Scaling: Fill --

    #[test]
    fn scale_fill_uses_exact_target() {
        let p = ImagePlacement::new(1)
            .size(10, 8)
            .scale_mode(ScaleMode::Fill);
        let s = compute_scaling(&p, 200, 100, 8.0, 16.0);
        assert_eq!(s.dst_cols, 10);
        assert_eq!(s.dst_rows, 8);
        assert_eq!(s.offset_col, 0);
        assert_eq!(s.offset_row, 0);
    }

    // -- Scaling: None --

    #[test]
    fn scale_none_native_size() {
        let p = ImagePlacement::new(1)
            .size(100, 100)
            .scale_mode(ScaleMode::None);
        // 80px wide image at 8px/cell = 10 cells, 48px tall at 16px/cell = 3 cells.
        let s = compute_scaling(&p, 80, 48, 8.0, 16.0);
        assert_eq!(s.dst_cols, 10);
        assert_eq!(s.dst_rows, 3);
    }

    #[test]
    fn scale_none_clipped_to_target() {
        let p = ImagePlacement::new(1)
            .size(5, 2)
            .scale_mode(ScaleMode::None);
        // Native would be 10x3 cells, but target is 5x2.
        let s = compute_scaling(&p, 80, 48, 8.0, 16.0);
        assert_eq!(s.dst_cols, 5);
        assert_eq!(s.dst_rows, 2);
    }

    // -- Scaling: Contain --

    #[test]
    fn scale_contain_wider_image() {
        // Image 200x100 px, target 10x10 cells, cell 8x16 px.
        // Target area: 80x160 px. Image aspect 2:1, target aspect 0.5:1.
        // Image is wider -> fit to width: 80px wide, 40px tall -> 10 cols, 3 rows.
        let p = ImagePlacement::new(1).size(10, 10);
        let s = compute_scaling(&p, 200, 100, 8.0, 16.0);
        assert_eq!(s.dst_cols, 10);
        assert!(s.dst_rows <= 10);
        // Should be centered vertically.
        assert!(s.offset_row > 0 || s.dst_rows == 10);
    }

    #[test]
    fn scale_contain_taller_image() {
        // Image 100x400 px, target 20x10 cells, cell 8x16 px.
        // Target area: 160x160 px. Image aspect 0.25:1, target aspect 1:1.
        // Image is taller -> fit to height: 160px tall, 40px wide -> 5 cols, 10 rows.
        let p = ImagePlacement::new(1).size(20, 10);
        let s = compute_scaling(&p, 100, 400, 8.0, 16.0);
        assert!(s.dst_cols <= 20);
        assert_eq!(s.dst_rows, 10);
        // Should be centered horizontally.
        assert!(s.offset_col > 0 || s.dst_cols == 20);
    }

    // -- Scaling: Cover --

    #[test]
    fn scale_cover_fills_target() {
        let p = ImagePlacement::new(1)
            .size(10, 10)
            .scale_mode(ScaleMode::Cover);
        let s = compute_scaling(&p, 200, 100, 8.0, 16.0);
        assert_eq!(s.dst_cols, 10);
        assert_eq!(s.dst_rows, 10);
        assert_eq!(s.offset_col, 0);
        assert_eq!(s.offset_row, 0);
    }

    #[test]
    fn scale_cover_crops_source() {
        let p = ImagePlacement::new(1)
            .size(10, 10)
            .scale_mode(ScaleMode::Cover);
        // Image 200x100, target 80x160 px (10x10 cells at 8x16).
        // Target aspect 0.5, source aspect 2.0 -> source is wider -> crop horizontally.
        let s = compute_scaling(&p, 200, 100, 8.0, 16.0);
        // Source should be cropped: visible width < 200.
        assert!(s.src_w <= 200);
        assert_eq!(s.src_h, 100);
    }

    // -- Cropping --

    #[test]
    fn scaling_with_source_rect() {
        let p = ImagePlacement::new(1)
            .size(10, 5)
            .crop(SourceRect::new(50, 25, 100, 50));
        let s = compute_scaling(&p, 200, 100, 8.0, 16.0);
        assert_eq!(s.src_x, 50);
        assert_eq!(s.src_y, 25);
    }

    // -- Auto size (no explicit size) --

    #[test]
    fn auto_size_from_image_dimensions() {
        let p = ImagePlacement::new(1);
        // 80px wide at 8px/cell = 10 cells, 48px tall at 16px/cell = 3 cells.
        let s = compute_scaling(&p, 80, 48, 8.0, 16.0);
        assert_eq!(s.dst_cols, 10);
        assert_eq!(s.dst_rows, 3);
    }

    // -- Encoding --

    #[test]
    fn encode_placement_basic() {
        let p = ImagePlacement::new(42).at(5, 3);
        let s = ScaledRegion {
            src_x: 0,
            src_y: 0,
            src_w: 100,
            src_h: 50,
            dst_cols: 10,
            dst_rows: 5,
            offset_col: 0,
            offset_row: 0,
        };
        let bytes = encode_placement(&p, &s);
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.starts_with("\x1b_G"));
        assert!(text.ends_with("\x1b\\"));
        assert!(text.contains("a=p"));
        assert!(text.contains("i=42"));
        assert!(text.contains("c=10"));
        assert!(text.contains("r=5"));
    }

    #[test]
    fn encode_placement_with_crop() {
        let p = ImagePlacement::new(1)
            .crop(SourceRect::new(10, 20, 50, 30))
            .size(5, 3);
        let s = ScaledRegion {
            src_x: 10,
            src_y: 20,
            src_w: 50,
            src_h: 30,
            dst_cols: 5,
            dst_rows: 3,
            offset_col: 0,
            offset_row: 0,
        };
        let bytes = encode_placement(&p, &s);
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("x=10"));
        assert!(text.contains("y=20"));
        assert!(text.contains("w=50"));
        assert!(text.contains("h=30"));
    }

    #[test]
    fn encode_placement_cursor_suppress() {
        let p = ImagePlacement::new(1).cursor_movement(CursorMovement::Suppress);
        let s = ScaledRegion {
            src_x: 0,
            src_y: 0,
            src_w: 10,
            src_h: 10,
            dst_cols: 2,
            dst_rows: 2,
            offset_col: 0,
            offset_row: 0,
        };
        let bytes = encode_placement(&p, &s);
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("C=1"));
    }

    #[test]
    fn encode_placement_z_index() {
        let p = ImagePlacement::new(1).with_z_index(-5);
        let s = ScaledRegion {
            src_x: 0,
            src_y: 0,
            src_w: 10,
            src_h: 10,
            dst_cols: 2,
            dst_rows: 2,
            offset_col: 0,
            offset_row: 0,
        };
        let bytes = encode_placement(&p, &s);
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("z=-5"));
    }

    #[test]
    fn encode_placement_with_placement_id() {
        let p = ImagePlacement::new(1).with_placement_id(99);
        let s = ScaledRegion {
            src_x: 0,
            src_y: 0,
            src_w: 10,
            src_h: 10,
            dst_cols: 2,
            dst_rows: 2,
            offset_col: 0,
            offset_row: 0,
        };
        let bytes = encode_placement(&p, &s);
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("p=99"));
    }

    // -- Cursor move encoding --

    #[test]
    fn encode_cursor_move_1based() {
        let bytes = encode_cursor_move(0, 0);
        assert_eq!(String::from_utf8(bytes).unwrap(), "\x1b[1;1H");

        let bytes = encode_cursor_move(5, 10);
        assert_eq!(String::from_utf8(bytes).unwrap(), "\x1b[11;6H");
    }

    // -- Full placement encoding --

    #[test]
    fn encode_full_placement_includes_cursor_move() {
        let p = ImagePlacement::new(1).at(3, 7);
        let s = ScaledRegion {
            src_x: 0,
            src_y: 0,
            src_w: 10,
            src_h: 10,
            dst_cols: 2,
            dst_rows: 2,
            offset_col: 0,
            offset_row: 0,
        };
        let bytes = encode_full_placement(&p, &s);
        let text = String::from_utf8(bytes).unwrap();
        // Should start with cursor move to (3,7) -> ESC[8;4H
        assert!(text.starts_with("\x1b[8;4H"));
        // Followed by placement command.
        assert!(text.contains("\x1b_G"));
    }

    #[test]
    fn encode_full_placement_with_offset() {
        let p = ImagePlacement::new(1).at(3, 7);
        let s = ScaledRegion {
            src_x: 0,
            src_y: 0,
            src_w: 10,
            src_h: 10,
            dst_cols: 2,
            dst_rows: 2,
            offset_col: 2,
            offset_row: 1,
        };
        let bytes = encode_full_placement(&p, &s);
        let text = String::from_utf8(bytes).unwrap();
        // Effective position: col=3+2=5, row=7+1=8 -> ESC[9;6H
        assert!(text.starts_with("\x1b[9;6H"));
    }

    // -- PlacementManager --

    #[test]
    fn manager_add_and_get() {
        let mut mgr = PlacementManager::new();
        assert!(mgr.is_empty());

        let h = mgr.add(ImagePlacement::new(1).at(5, 5));
        assert_eq!(mgr.len(), 1);

        let p = mgr.get(h).unwrap();
        assert_eq!(p.col, 5);
        assert_eq!(p.row, 5);
    }

    #[test]
    fn manager_remove() {
        let mut mgr = PlacementManager::new();
        let h = mgr.add(ImagePlacement::new(1));
        let removed = mgr.remove(h);
        assert!(removed.is_some());
        assert!(mgr.is_empty());
        assert!(mgr.remove(h).is_none());
    }

    #[test]
    fn manager_get_mut() {
        let mut mgr = PlacementManager::new();
        let h = mgr.add(ImagePlacement::new(1).at(0, 0));
        if let Some(p) = mgr.get_mut(h) {
            p.col = 10;
        }
        assert_eq!(mgr.get(h).unwrap().col, 10);
    }

    #[test]
    fn manager_sorted_by_layer_and_z() {
        let mut mgr = PlacementManager::new();
        let h1 = mgr.add(
            ImagePlacement::new(1)
                .on_layer(ImageLayer::AboveText)
                .with_z_index(5),
        );
        let h2 = mgr.add(
            ImagePlacement::new(2)
                .on_layer(ImageLayer::BelowText)
                .with_z_index(10),
        );
        let h3 = mgr.add(
            ImagePlacement::new(3)
                .on_layer(ImageLayer::AboveText)
                .with_z_index(1),
        );

        let sorted = mgr.sorted_placements();
        // BelowText first, then AboveText sorted by z.
        assert_eq!(sorted[0].0, h2); // BelowText, z=10
        assert_eq!(sorted[1].0, h3); // AboveText, z=1
        assert_eq!(sorted[2].0, h1); // AboveText, z=5
    }

    #[test]
    fn manager_multiple_placements_same_image() {
        let mut mgr = PlacementManager::new();
        let h1 = mgr.add(ImagePlacement::new(42).at(0, 0));
        let h2 = mgr.add(ImagePlacement::new(42).at(10, 5));
        let h3 = mgr.add(ImagePlacement::new(42).at(20, 10));

        assert_eq!(mgr.len(), 3);
        assert_eq!(mgr.get(h1).unwrap().image_id, 42);
        assert_eq!(mgr.get(h2).unwrap().image_id, 42);
        assert_eq!(mgr.get(h3).unwrap().image_id, 42);

        // Different positions.
        assert_eq!(mgr.get(h1).unwrap().col, 0);
        assert_eq!(mgr.get(h2).unwrap().col, 10);
        assert_eq!(mgr.get(h3).unwrap().col, 20);
    }

    #[test]
    fn manager_clear() {
        let mut mgr = PlacementManager::new();
        mgr.add(ImagePlacement::new(1));
        mgr.add(ImagePlacement::new(2));
        mgr.clear();
        assert!(mgr.is_empty());
    }

    #[test]
    fn manager_register_image_dimensions() {
        let mut mgr = PlacementManager::new();
        mgr.register_image(1, 200, 100);
        assert_eq!(mgr.image_dimensions(1), Some((200, 100)));
        assert_eq!(mgr.image_dimensions(99), None);
    }

    #[test]
    fn manager_render_all_produces_output() {
        let mut mgr = PlacementManager::new();
        mgr.register_image(1, 80, 48);
        mgr.add(ImagePlacement::new(1).at(0, 0).size(10, 3));
        mgr.add(ImagePlacement::new(1).at(15, 5).size(10, 3));

        let output = mgr.render_all(8.0, 16.0);
        let text = String::from_utf8(output).unwrap();

        // Should contain two cursor moves and two placement commands.
        let cup_count = text.matches("\x1b[").count();
        let apc_count = text.matches("\x1b_G").count();
        assert_eq!(apc_count, 2);
        assert!(cup_count >= 2);
    }

    // -- Layout integration --

    #[test]
    fn place_in_layout_basic() {
        let layout = ComputedLayout {
            x: 5.0,
            y: 10.0,
            width: 20.0,
            height: 8.0,
        };
        let p = place_in_layout(42, &layout, ScaleMode::Contain, ImageLayer::AboveText);
        assert_eq!(p.image_id, 42);
        assert_eq!(p.col, 5);
        assert_eq!(p.row, 10);
        assert_eq!(p.width_cells, Some(20));
        assert_eq!(p.height_cells, Some(8));
        assert_eq!(p.scale, ScaleMode::Contain);
        assert_eq!(p.layer, ImageLayer::AboveText);
    }

    #[test]
    fn place_in_layout_rounds_position() {
        let layout = ComputedLayout {
            x: 5.7,
            y: 10.3,
            width: 20.5,
            height: 8.4,
        };
        let p = place_in_layout(1, &layout, ScaleMode::Fill, ImageLayer::BelowText);
        assert_eq!(p.col, 6); // 5.7 rounds to 6
        assert_eq!(p.row, 10); // 10.3 rounds to 10
        assert_eq!(p.width_cells, Some(21)); // 20.5 rounds to 21
        assert_eq!(p.height_cells, Some(8)); // 8.4 rounds to 8
    }

    #[test]
    fn place_in_layout_min_size() {
        let layout = ComputedLayout {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        };
        let p = place_in_layout(1, &layout, ScaleMode::None, ImageLayer::AboveText);
        // Should clamp to at least 1x1.
        assert_eq!(p.width_cells, Some(1));
        assert_eq!(p.height_cells, Some(1));
    }

    // -- Edge cases --

    #[test]
    fn scale_with_very_small_cell_size() {
        // Very small cell size should not panic.
        let p = ImagePlacement::new(1).size(10, 10);
        let s = compute_scaling(&p, 100, 100, 0.01, 0.01);
        assert!(s.dst_cols > 0);
        assert!(s.dst_rows > 0);
    }

    #[test]
    fn encode_placement_no_z_index_if_zero() {
        let p = ImagePlacement::new(1).with_z_index(0);
        let s = ScaledRegion {
            src_x: 0,
            src_y: 0,
            src_w: 10,
            src_h: 10,
            dst_cols: 2,
            dst_rows: 2,
            offset_col: 0,
            offset_row: 0,
        };
        let bytes = encode_placement(&p, &s);
        let text = String::from_utf8(bytes).unwrap();
        assert!(!text.contains("z="));
    }

    #[test]
    fn encode_placement_no_cursor_suppress_by_default() {
        let p = ImagePlacement::new(1);
        let s = ScaledRegion {
            src_x: 0,
            src_y: 0,
            src_w: 10,
            src_h: 10,
            dst_cols: 2,
            dst_rows: 2,
            offset_col: 0,
            offset_row: 0,
        };
        let bytes = encode_placement(&p, &s);
        let text = String::from_utf8(bytes).unwrap();
        assert!(!text.contains("C=1"));
    }
}
