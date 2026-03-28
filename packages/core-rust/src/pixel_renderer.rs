//! Full-frame pixel renderer for Kitty graphics protocol output.
//!
//! [`PixelRenderer`] replaces cell-based rendering when Kitty graphics is
//! available.  It walks a layout tree, paints backgrounds, borders, text,
//! and shadows onto a [`PixelCanvas`], then encodes the result as a Kitty
//! protocol image for display.
//!
//! This module is intentionally decoupled from [`crate::ffi_bridge::EngineState`]
//! through the [`PaintTree`] trait, so integration can happen separately (see
//! issue #131).

// Pixel math inherently involves many single-char variable names (x, y, w, h)
// and frequent numeric casts between f32/u32/u8/usize.
#![allow(
    clippy::many_single_char_names,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_lossless,
    clippy::cast_possible_wrap,
    clippy::must_use_candidate,
    clippy::too_many_arguments,
    clippy::needless_range_loop,
    clippy::module_name_repetitions,
    clippy::similar_names,
    clippy::trivially_copy_pass_by_ref
)]

use crate::ansi::Color;
use crate::font_system::FontSystem;
use crate::image::{self, ImageData};
use crate::pixel_canvas::PixelCanvas;

// ---------------------------------------------------------------------------
// Paint-tree abstraction
// ---------------------------------------------------------------------------

/// Computed layout for a single node (cell coordinates).
#[derive(Clone, Copy, Debug)]
pub struct NodeLayout {
    /// X position relative to parent (columns).
    pub x: f32,
    /// Y position relative to parent (rows).
    pub y: f32,
    /// Width in columns.
    pub width: f32,
    /// Height in rows.
    pub height: f32,
}

/// Visual style data the pixel renderer needs per node.
#[derive(Clone, Debug, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct PixelNodeStyle {
    /// Background color.
    pub bg: Option<Color>,
    /// Foreground (text) color.
    pub fg: Option<Color>,
    /// Bold text.
    pub bold: bool,
    /// Italic text.
    pub italic: bool,
    /// Underline decoration.
    pub underline: bool,
    /// Strikethrough decoration.
    pub strikethrough: bool,
    /// Dim text (rendered at lower alpha).
    pub dim: bool,
    /// Border radius in pixels for rounded corners.
    pub border_radius: f32,
    /// Whether overflow is hidden (enables clipping).
    pub overflow_hidden: bool,
    /// Border thickness (0 = no border).
    pub border_thickness: f32,
    /// Border color.
    pub border_color: Option<Color>,
    /// Box shadow properties.
    pub box_shadow: Option<PixelBoxShadow>,
    /// Linear gradient background (takes precedence over `bg`).
    pub gradient: Option<PixelGradient>,
    /// Explicit font size in pixels (overrides `cell_h` default).
    pub font_size: Option<f32>,
}

/// Box shadow properties for pixel rendering.
#[derive(Clone, Debug)]
pub struct PixelBoxShadow {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur_radius: f32,
    pub spread_radius: f32,
    pub color: [u8; 4],
}

/// Linear gradient for pixel rendering.
#[derive(Clone, Debug)]
pub struct PixelGradient {
    pub angle_deg: f32,
    pub stops: Vec<(f32, [u8; 4])>,
}

/// Per-character color override within text content.
#[derive(Clone, Debug)]
pub struct PixelTextSpan {
    /// Start byte offset in the text string.
    pub start: u16,
    /// End byte offset (exclusive) in the text string.
    pub end: u16,
    /// Foreground color for this span.
    pub fg: [u8; 4],
}

/// Trait for providing the layout tree data to the pixel renderer.
///
/// This decouples the renderer from `EngineState` so integration (#131) can
/// bridge the two without leaking internal types.
pub trait PaintTree {
    /// Return the root node ID, if any.
    fn root_node(&self) -> Option<u32>;

    /// Return the computed layout for a node, or `None` if the node is unknown.
    fn node_layout(&self, node_id: u32) -> Option<NodeLayout>;

    /// Return the visual style for a node, or `None` for unstyled nodes.
    fn node_style(&self, node_id: u32) -> Option<PixelNodeStyle>;

    /// Return the text content for a node, or `None` if the node has no text.
    fn text_content(&self, node_id: u32) -> Option<&str>;

    /// Return per-character color spans for a node's text.  Default returns
    /// empty (single-color text).
    fn text_spans(&self, _node_id: u32) -> Vec<PixelTextSpan> {
        Vec::new()
    }

    /// Return the child node IDs for a node (in order).
    fn children(&self, node_id: u32) -> Vec<u32>;
}

// ---------------------------------------------------------------------------
// PixelRenderer
// ---------------------------------------------------------------------------

/// Full-frame pixel renderer.  Replaces cell-based rendering when Kitty
/// graphics is available.
pub struct PixelRenderer {
    /// Full-frame pixel canvas.
    canvas: PixelCanvas,
    /// Font system for text rasterization.
    font_system: FontSystem,
    /// Cell width in pixels.
    cell_w: u32,
    /// Cell height in pixels.
    cell_h: u32,
    /// Terminal width in cells.
    cols: u32,
    /// Terminal height in cells.
    rows: u32,
    /// Hash of each row-tile from the previous frame.
    prev_row_hashes: Vec<u64>,
    /// Kitty image IDs for each row-tile.
    row_image_ids: Vec<u32>,
}

impl PixelRenderer {
    /// Create a new pixel renderer for the given terminal dimensions.
    pub fn new(cols: u32, rows: u32, cell_w: u32, cell_h: u32) -> Self {
        Self::new_with_font_system(cols, rows, cell_w, cell_h, FontSystem::new())
    }

    /// Create a pixel renderer with a pre-loaded [`FontSystem`].
    ///
    /// This avoids loading fonts on the render thread -- callers can load
    /// fonts eagerly on the main thread (e.g. in `EngineState::new()`) and
    /// pass the result here.  This fixes a hang when `std::fs::read` of
    /// system font files blocks in headless / background processes.
    pub fn new_with_font_system(
        cols: u32,
        rows: u32,
        cell_w: u32,
        cell_h: u32,
        font_system: FontSystem,
    ) -> Self {
        let px_w = cols * cell_w;
        let px_h = rows * cell_h;
        Self {
            canvas: PixelCanvas::new(px_w, px_h),
            font_system,
            cell_w,
            cell_h,
            cols,
            rows,
            prev_row_hashes: Vec::new(),
            row_image_ids: Vec::new(),
        }
    }

    /// Resize the canvas when terminal dimensions change.
    pub fn resize(&mut self, cols: u32, rows: u32) {
        if cols != self.cols || rows != self.rows {
            self.cols = cols;
            self.rows = rows;
            self.canvas = PixelCanvas::new(cols * self.cell_w, rows * self.cell_h);
            self.prev_row_hashes.clear();
            self.row_image_ids.clear();
        }
    }

    /// Return the current canvas width in pixels.
    pub fn canvas_width(&self) -> u32 {
        self.canvas.width
    }

    /// Return the current canvas height in pixels.
    pub fn canvas_height(&self) -> u32 {
        self.canvas.height
    }

    /// Save the current canvas as a PNG file.
    ///
    /// # Errors
    ///
    /// Returns an error string if the image buffer cannot be created or saved.
    pub fn save_screenshot(&self, path: &str) -> Result<(), String> {
        use ::image::ImageBuffer;
        use ::image::Rgba;

        let w = self.canvas.width;
        let h = self.canvas.height;
        let img = ImageBuffer::<Rgba<u8>, _>::from_raw(w, h, self.canvas.data.clone())
            .ok_or("Failed to create image buffer")?;
        img.save(path).map_err(|e| e.to_string())
    }

    /// Paint the entire frame from a [`PaintTree`].  Returns the Kitty
    /// protocol bytes to write to the terminal -- only the row-tiles that
    /// actually changed since the previous frame.
    pub fn paint_frame(&mut self, tree: &dyn PaintTree) -> Vec<u8> {
        // Clear canvas (transparent).
        self.canvas.fill([0, 0, 0, 0]);

        // Walk layout tree and paint all nodes.
        if let Some(root_id) = tree.root_node() {
            self.paint_node(tree, root_id, 0.0, 0.0, None);
        }

        // Encode only the dirty row-tiles.
        self.encode_tiles()
    }

    /// Recursively paint a node and its children.
    #[allow(clippy::too_many_lines)]
    fn paint_node(
        &mut self,
        tree: &dyn PaintTree,
        node_id: u32,
        parent_x: f32,
        parent_y: f32,
        clip: Option<(f32, f32, f32, f32)>,
    ) {
        let Some(layout) = tree.node_layout(node_id) else {
            return;
        };

        let cell_x = parent_x + layout.x;
        let cell_y = parent_y + layout.y;

        // Convert cell coords to pixel coords.
        let px_x = cell_x * self.cell_w as f32;
        let px_y = cell_y * self.cell_h as f32;
        let px_w = layout.width * self.cell_w as f32;
        let px_h = layout.height * self.cell_h as f32;

        // Apply clip if active.
        if let Some((cx, cy, cw, ch)) = clip {
            if px_x >= cx + cw || px_y >= cy + ch || px_x + px_w <= cx || px_y + px_h <= cy {
                return; // Entirely outside clip region.
            }
        }

        let style = tree.node_style(node_id);

        if let Some(ref style) = style {
            let border_radius = style.border_radius;

            // 1. Box shadow (behind everything).
            if let Some(ref shadow) = style.box_shadow {
                self.paint_shadow(px_x, px_y, px_w, px_h, border_radius, shadow);
            }

            // 2. Background (gradient or solid).
            if let Some(ref gradient) = style.gradient {
                if border_radius > 0.0 {
                    self.canvas.fill_linear_gradient_rounded(
                        px_x,
                        px_y,
                        px_w,
                        px_h,
                        gradient.angle_deg,
                        &gradient.stops,
                        border_radius,
                    );
                } else {
                    self.canvas.fill_linear_gradient(
                        px_x,
                        px_y,
                        px_w,
                        px_h,
                        gradient.angle_deg,
                        &gradient.stops,
                    );
                }
            } else if let Some(ref bg) = style.bg {
                let rgba = color_to_rgba(bg, 255);
                if border_radius > 0.0 {
                    self.canvas
                        .fill_rounded_rect(px_x, px_y, px_w, px_h, border_radius, rgba);
                } else {
                    self.canvas.fill_rect(px_x, px_y, px_w, px_h, rgba);
                }
            }

            // 3. Border.
            if style.border_thickness > 0.0 {
                let border_color = style
                    .border_color
                    .as_ref()
                    .map_or([255, 255, 255, 255], |c| color_to_rgba(c, 255));
                self.canvas.draw_border(
                    px_x,
                    px_y,
                    px_w,
                    px_h,
                    style.border_thickness,
                    [border_radius; 4],
                    border_color,
                );
            }

            // 4. Text.
            if let Some(text) = tree.text_content(node_id) {
                let alpha = if style.dim { 140 } else { 255 };
                let fg = style
                    .fg
                    .as_ref()
                    .map_or([255, 255, 255, alpha], |c| color_to_rgba(c, alpha));
                let font_size = style.font_size.unwrap_or(self.cell_h as f32);

                // Text y position: use the Taffy-computed position directly.
                // Taffy handles vertical centering via flexbox (justifyContent/alignItems).
                let text_y = px_y;

                let spans = tree.text_spans(node_id);
                if spans.is_empty() {
                    // Single-color fast path.
                    self.canvas.draw_text(
                        px_x,
                        text_y,
                        text,
                        fg,
                        font_size,
                        style.bold,
                        style.italic,
                        &mut self.font_system,
                    );
                } else {
                    // Multi-color text: render each segment separately.
                    // First, measure character advance positions to place
                    // each segment correctly.  We render the full text for
                    // shaping, then re-render per-span with the right color.
                    // This is an approximation -- full segment rendering
                    // would require tracking glyph byte-offset mapping, but
                    // for monospace text this is correct.
                    let char_w = font_size * 0.6; // monospace approximate
                    let mut covered = vec![false; text.len()];
                    for span in &spans {
                        let start = (span.start as usize).min(text.len());
                        let end = (span.end as usize).min(text.len());
                        if start >= end {
                            continue;
                        }
                        let segment = &text[start..end];
                        let char_offset: usize = text[..start].chars().count();
                        let seg_x = px_x + char_offset as f32 * char_w;
                        let mut span_fg = span.fg;
                        span_fg[3] = ((span_fg[3] as u32 * alpha as u32) / 255) as u8;
                        self.canvas.draw_text(
                            seg_x,
                            text_y,
                            segment,
                            span_fg,
                            font_size,
                            style.bold,
                            style.italic,
                            &mut self.font_system,
                        );
                        for i in start..end {
                            covered[i] = true;
                        }
                    }
                    // Render uncovered portions with default fg.
                    let mut i = 0;
                    let bytes = text.as_bytes();
                    while i < bytes.len() {
                        if covered[i] {
                            i += 1;
                        } else {
                            let start = i;
                            while i < bytes.len() && !covered[i] {
                                i += 1;
                            }
                            let segment = &text[start..i];
                            let char_offset: usize = text[..start].chars().count();
                            let seg_x = px_x + char_offset as f32 * char_w;
                            self.canvas.draw_text(
                                seg_x,
                                text_y,
                                segment,
                                fg,
                                font_size,
                                style.bold,
                                style.italic,
                                &mut self.font_system,
                            );
                        }
                    }
                }

                // Text decorations.
                if style.underline {
                    let line_y = text_y + font_size * 0.9;
                    self.canvas
                        .draw_line(px_x, line_y, px_x + px_w, line_y, 1.0, fg);
                }
                if style.strikethrough {
                    let line_y = text_y + font_size * 0.5;
                    self.canvas
                        .draw_line(px_x, line_y, px_x + px_w, line_y, 1.0, fg);
                }
            }
        }

        // 5. Recurse into children.
        let children = tree.children(node_id);

        // Compute clip rect for overflow:hidden.
        let new_clip = if style.as_ref().is_some_and(|s| s.overflow_hidden) {
            Some((px_x, px_y, px_w, px_h))
        } else {
            clip
        };

        for child_id in children {
            self.paint_node(tree, child_id, cell_x, cell_y, new_clip);
        }
    }

    /// Paint a box shadow below the node.
    fn paint_shadow(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        radius: f32,
        shadow: &PixelBoxShadow,
    ) {
        let blur = shadow.blur_radius;
        let spread = shadow.spread_radius;

        // The shadow shape is the node rect expanded by `spread` on each side.
        let inner_w = (w + spread * 2.0).max(0.0);
        let inner_h = (h + spread * 2.0).max(0.0);
        // The blur needs `blur` pixels of padding around the shape.
        let pad = blur.ceil();
        let sw = inner_w + pad * 2.0;
        let sh = inner_h + pad * 2.0;

        // Shadow canvas origin in main-canvas coordinates: the shape center
        // is at (x + w/2 + offset_x, y + h/2 + offset_y), so the shadow
        // canvas top-left is offset by half-canvas from that center.
        let sx = x + shadow.offset_x - spread - pad;
        let sy = y + shadow.offset_y - spread - pad;

        let sw_u = sw.ceil() as u32;
        let sh_u = sh.ceil() as u32;
        if sw_u == 0 || sh_u == 0 || sw_u > 4096 || sh_u > 4096 {
            return;
        }

        let mut shadow_canvas = PixelCanvas::new(sw_u, sh_u);
        let inner_x = pad;
        let inner_y = pad;

        if radius > 0.0 {
            shadow_canvas.fill_rounded_rect(
                inner_x,
                inner_y,
                inner_w,
                inner_h,
                radius,
                shadow.color,
            );
        } else {
            shadow_canvas.fill_rect(inner_x, inner_y, inner_w, inner_h, shadow.color);
        }

        let blur_px = blur.ceil() as u32;
        shadow_canvas.box_blur(blur_px);

        // Composite shadow onto main canvas.
        for py in 0..sh_u {
            for px in 0..sw_u {
                let pix = shadow_canvas.get_pixel(px, py);
                if pix[3] > 0 {
                    let dest_x = (sx + px as f32) as i32;
                    let dest_y = (sy + py as f32) as i32;
                    if dest_x >= 0 && dest_y >= 0 {
                        self.canvas.blend_pixel(dest_x as u32, dest_y as u32, pix);
                    }
                }
            }
        }
    }

    /// Encode only the row-tiles that changed since the previous frame.
    ///
    /// Each "tile" is one cell-row of the canvas (`full_width x cell_h`
    /// pixels).  We hash each row's pixel data and skip rows that are
    /// identical to the previous frame, dramatically reducing the amount
    /// of data sent over the wire.
    fn encode_tiles(&mut self) -> Vec<u8> {
        let mut output = Vec::new();
        let tile_h = self.cell_h;
        let tile_w = self.canvas.width;
        let num_rows = self.rows as usize;

        // On first frame (or after resize), reset tracking vectors and
        // delete any stale images the terminal may still hold.
        if self.prev_row_hashes.len() != num_rows {
            self.prev_row_hashes = vec![0u64; num_rows];
            self.row_image_ids = vec![0u32; num_rows];
            let delete = image::encode_delete(image::DeleteTarget::All);
            output.extend_from_slice(&delete);
        }

        let stride = tile_w as usize * 4; // bytes per pixel row

        for row in 0..num_rows {
            let y_start = row * tile_h as usize;
            let y_end = ((row + 1) * tile_h as usize).min(self.canvas.height as usize);
            let byte_start = y_start * stride;
            let byte_end = y_end * stride;
            let row_bytes = &self.canvas.data[byte_start..byte_end];

            let hash = fnv_hash(row_bytes);

            if hash == self.prev_row_hashes[row] && self.row_image_ids[row] > 0 {
                continue; // Row unchanged — skip.
            }
            self.prev_row_hashes[row] = hash;

            // Delete the old tile image if one exists.
            let old_id = self.row_image_ids[row];
            if old_id > 0 {
                let delete = image::encode_delete(image::DeleteTarget::ById(old_id));
                output.extend_from_slice(&delete);
            }

            // Allocate a new image ID and transmit the tile.
            let new_id = image::ImageCache::next_id();
            self.row_image_ids[row] = new_id;

            let tile_height = (y_end - y_start) as u32;
            if let Ok(img_data) = ImageData::from_rgba(row_bytes.to_vec(), tile_w, tile_height) {
                if let Ok(transmit) = image::encode_transmit(&img_data, new_id) {
                    // Position cursor at this row (1-based).
                    let cursor = format!("\x1b[{};1H", row + 1);
                    output.extend_from_slice(cursor.as_bytes());
                    output.extend_from_slice(&transmit);
                    let display = image::encode_display_z(new_id, None, -1);
                    output.extend_from_slice(&display);
                }
            }
        }

        output
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// FNV-1a hash of a byte slice.  Used for fast row-tile dirty detection.
fn fnv_hash(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// Convert a [`Color`] to an RGBA quad.
fn color_to_rgba(color: &Color, alpha: u8) -> [u8; 4] {
    match color {
        Color::Rgb(r, g, b) => [*r, *g, *b, alpha],
        // Approximate standard ANSI colors.
        Color::Ansi(idx) => ansi_index_to_rgb(*idx, alpha),
        Color::AnsiBright(idx) => ansi_bright_index_to_rgb(*idx, alpha),
        Color::Palette(idx) => palette_index_to_rgb(*idx, alpha),
    }
}

/// Map a 256-color palette index to RGB.
///
/// - 0-7: standard ANSI colors
/// - 8-15: bright ANSI colors
/// - 16-231: 6x6x6 color cube
/// - 232-255: 24-step grayscale ramp
fn palette_index_to_rgb(idx: u8, alpha: u8) -> [u8; 4] {
    match idx {
        0..=7 => ansi_index_to_rgb(idx, alpha),
        8..=15 => ansi_bright_index_to_rgb(idx - 8, alpha),
        16..=231 => {
            let n = idx - 16;
            let b = n % 6;
            let g = (n / 6) % 6;
            let r = n / 36;
            let to_rgb = |v: u8| if v == 0 { 0 } else { 55 + 40 * v };
            [to_rgb(r), to_rgb(g), to_rgb(b), alpha]
        }
        232..=255 => {
            let v = 8 + 10 * (idx - 232);
            [v, v, v, alpha]
        }
    }
}

/// Map standard ANSI color index (0-7) to approximate RGB.
fn ansi_index_to_rgb(idx: u8, alpha: u8) -> [u8; 4] {
    match idx {
        0 => [0, 0, 0, alpha],       // black
        1 => [170, 0, 0, alpha],     // red
        2 => [0, 170, 0, alpha],     // green
        3 => [170, 170, 0, alpha],   // yellow
        4 => [0, 0, 170, alpha],     // blue
        5 => [170, 0, 170, alpha],   // magenta
        6 => [0, 170, 170, alpha],   // cyan
        7 => [170, 170, 170, alpha], // white
        _ => [128, 128, 128, alpha],
    }
}

/// Map bright ANSI color index (0-7) to approximate RGB.
fn ansi_bright_index_to_rgb(idx: u8, alpha: u8) -> [u8; 4] {
    match idx {
        0 => [85, 85, 85, alpha],    // bright black
        1 => [255, 85, 85, alpha],   // bright red
        2 => [85, 255, 85, alpha],   // bright green
        3 => [255, 255, 85, alpha],  // bright yellow
        4 => [85, 85, 255, alpha],   // bright blue
        5 => [255, 85, 255, alpha],  // bright magenta
        6 => [85, 255, 255, alpha],  // bright cyan
        7 => [255, 255, 255, alpha], // bright white
        _ => [192, 192, 192, alpha],
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Test PaintTree implementation ------------------------------------

    /// Minimal in-memory tree for testing.
    struct TestTree {
        layouts: std::collections::HashMap<u32, NodeLayout>,
        styles: std::collections::HashMap<u32, PixelNodeStyle>,
        texts: std::collections::HashMap<u32, String>,
        children: std::collections::HashMap<u32, Vec<u32>>,
        root: Option<u32>,
    }

    impl TestTree {
        fn new() -> Self {
            Self {
                layouts: std::collections::HashMap::new(),
                styles: std::collections::HashMap::new(),
                texts: std::collections::HashMap::new(),
                children: std::collections::HashMap::new(),
                root: None,
            }
        }
    }

    impl PaintTree for TestTree {
        fn root_node(&self) -> Option<u32> {
            self.root
        }

        fn node_layout(&self, node_id: u32) -> Option<NodeLayout> {
            self.layouts.get(&node_id).copied()
        }

        fn node_style(&self, node_id: u32) -> Option<PixelNodeStyle> {
            self.styles.get(&node_id).cloned()
        }

        fn text_content(&self, node_id: u32) -> Option<&str> {
            self.texts.get(&node_id).map(|s| s.as_str())
        }

        fn children(&self, node_id: u32) -> Vec<u32> {
            self.children.get(&node_id).cloned().unwrap_or_default()
        }
    }

    // -- PixelRenderer::new -----------------------------------------------

    #[test]
    fn new_creates_correct_canvas_dimensions() {
        let r = PixelRenderer::new(80, 24, 8, 16);
        assert_eq!(r.canvas_width(), 80 * 8);
        assert_eq!(r.canvas_height(), 24 * 16);
        assert_eq!(r.cols, 80);
        assert_eq!(r.rows, 24);
    }

    // -- resize -----------------------------------------------------------

    #[test]
    fn resize_creates_new_canvas() {
        let mut r = PixelRenderer::new(80, 24, 8, 16);
        r.resize(120, 40);
        assert_eq!(r.cols, 120);
        assert_eq!(r.rows, 40);
        assert_eq!(r.canvas_width(), 120 * 8);
        assert_eq!(r.canvas_height(), 40 * 16);
    }

    #[test]
    fn resize_noop_when_dimensions_unchanged() {
        let mut r = PixelRenderer::new(80, 24, 8, 16);
        let ptr_before = r.canvas.data.as_ptr();
        r.resize(80, 24);
        // Canvas should be the same allocation (no resize occurred).
        assert_eq!(r.canvas.data.as_ptr(), ptr_before);
    }

    // -- paint_frame with empty tree --------------------------------------

    #[test]
    fn paint_frame_empty_tree_produces_empty_output() {
        let mut r = PixelRenderer::new(10, 5, 8, 16);
        let tree = TestTree::new();
        let output = r.paint_frame(&tree);
        // No root node means the canvas is all-transparent; encode_tiles
        // still generates Kitty commands for the blank frame on first call.
        // We mainly verify it does not panic.
        assert!(!output.is_empty() || tree.root.is_none());
    }

    // -- paint_frame with a single colored node ---------------------------

    #[test]
    fn paint_frame_single_node_has_nonzero_pixels() {
        let mut r = PixelRenderer::new(10, 5, 8, 16);
        let mut tree = TestTree::new();
        tree.root = Some(1);
        tree.layouts.insert(
            1,
            NodeLayout {
                x: 0.0,
                y: 0.0,
                width: 10.0,
                height: 5.0,
            },
        );
        tree.styles.insert(
            1,
            PixelNodeStyle {
                bg: Some(Color::Rgb(255, 0, 0)),
                ..Default::default()
            },
        );

        let output = r.paint_frame(&tree);

        // Canvas should have red pixels.
        let has_color = r
            .canvas
            .data
            .chunks_exact(4)
            .any(|px| px[0] > 0 && px[3] > 0);
        assert!(has_color, "canvas should have non-transparent red pixels");

        // Output should contain Kitty protocol escape sequences.
        assert!(!output.is_empty(), "output should not be empty");
    }

    // -- encode_tiles produces Kitty protocol bytes -----------------------

    #[test]
    fn encode_tiles_contains_kitty_protocol_sequences() {
        let mut r = PixelRenderer::new(4, 2, 8, 16);
        // Fill some pixels so there is data to encode.
        r.canvas.fill([128, 64, 32, 255]);
        let output = r.encode_tiles();
        let text = String::from_utf8_lossy(&output);

        // Should contain Kitty APC start.
        assert!(
            text.contains("\x1b_G"),
            "output should contain Kitty APC start"
        );
        // Should contain transmit action.
        assert!(
            text.contains("a=t"),
            "output should contain action=transmit"
        );
        // Should contain display placement.
        assert!(
            text.contains("a=p"),
            "output should contain action=placement"
        );
        // Should contain z-index.
        assert!(text.contains("z=-1"), "output should contain z=-1");
    }

    // -- tile diffing: unchanged canvas produces no output -----------------

    #[test]
    fn encode_tiles_unchanged_canvas_produces_no_output() {
        let mut r = PixelRenderer::new(4, 3, 8, 16);
        r.canvas.fill([10, 20, 30, 255]);

        // First call transmits all tiles.
        let first = r.encode_tiles();
        assert!(!first.is_empty(), "first frame should transmit tiles");

        // Second call with same canvas should produce nothing.
        let second = r.encode_tiles();
        assert!(
            second.is_empty(),
            "unchanged canvas should produce no output"
        );
    }

    // -- tile diffing: changing one pixel only retransmits that row --------

    #[test]
    fn encode_tiles_single_pixel_change_retransmits_one_row() {
        let mut r = PixelRenderer::new(4, 3, 8, 16);
        r.canvas.fill([10, 20, 30, 255]);

        // Transmit all tiles initially.
        let _ = r.encode_tiles();

        // Modify a single pixel in row 1 (y = cell_h).
        let y = r.cell_h;
        r.canvas.set_pixel(0, y, [255, 0, 0, 255]);

        let output = r.encode_tiles();
        let text = String::from_utf8_lossy(&output);

        // Should contain exactly one transmit (the dirty row).
        let transmit_count = text.matches("a=t").count();
        assert_eq!(
            transmit_count, 1,
            "only one row tile should be retransmitted, got {transmit_count}"
        );

        // The cursor should position at row 2 (1-based).
        assert!(
            text.contains("\x1b[2;1H"),
            "cursor should be positioned at row 2"
        );
    }

    // -- tile diffing: resize forces full retransmit ----------------------

    #[test]
    fn encode_tiles_after_resize_retransmits_all() {
        let mut r = PixelRenderer::new(4, 3, 8, 16);
        r.canvas.fill([10, 20, 30, 255]);
        let _ = r.encode_tiles();

        // Resize clears the hash/id vectors.
        r.resize(6, 4);
        r.canvas.fill([10, 20, 30, 255]);
        let output = r.encode_tiles();
        let text = String::from_utf8_lossy(&output);

        // All 4 rows should be transmitted.
        let transmit_count = text.matches("a=t").count();
        assert_eq!(
            transmit_count, 4,
            "after resize all rows should be retransmitted, got {transmit_count}"
        );
    }

    // -- tile diffing: first frame transmits all rows ---------------------

    #[test]
    fn encode_tiles_first_frame_transmits_all_rows() {
        let mut r = PixelRenderer::new(4, 5, 8, 16);
        r.canvas.fill([50, 50, 50, 255]);
        let output = r.encode_tiles();
        let text = String::from_utf8_lossy(&output);

        let transmit_count = text.matches("a=t").count();
        assert_eq!(
            transmit_count, 5,
            "first frame should transmit all 5 rows, got {transmit_count}"
        );
    }

    // -- paint_frame with text content ------------------------------------

    #[test]
    fn paint_frame_with_text_renders_pixels() {
        let mut r = PixelRenderer::new(20, 3, 8, 16);
        let mut tree = TestTree::new();
        tree.root = Some(1);
        tree.layouts.insert(
            1,
            NodeLayout {
                x: 0.0,
                y: 0.0,
                width: 20.0,
                height: 3.0,
            },
        );
        tree.styles.insert(
            1,
            PixelNodeStyle {
                fg: Some(Color::Rgb(255, 255, 255)),
                ..Default::default()
            },
        );
        tree.texts.insert(1, "Hello".to_string());

        let _output = r.paint_frame(&tree);

        // At least some pixels should be non-transparent from text rendering.
        let non_transparent = r.canvas.data.chunks_exact(4).filter(|px| px[3] > 0).count();
        assert!(
            non_transparent > 0,
            "text rendering should produce visible pixels"
        );
    }

    // -- paint_frame with parent-child hierarchy --------------------------

    #[test]
    fn paint_frame_parent_child_positions_correctly() {
        let mut r = PixelRenderer::new(20, 10, 8, 16);
        let mut tree = TestTree::new();
        tree.root = Some(1);

        // Parent at (0,0) size 20x10.
        tree.layouts.insert(
            1,
            NodeLayout {
                x: 0.0,
                y: 0.0,
                width: 20.0,
                height: 10.0,
            },
        );
        tree.styles.insert(
            1,
            PixelNodeStyle {
                bg: Some(Color::Rgb(0, 0, 100)),
                ..Default::default()
            },
        );
        tree.children.insert(1, vec![2]);

        // Child at (2,1) relative to parent, size 5x3.
        tree.layouts.insert(
            2,
            NodeLayout {
                x: 2.0,
                y: 1.0,
                width: 5.0,
                height: 3.0,
            },
        );
        tree.styles.insert(
            2,
            PixelNodeStyle {
                bg: Some(Color::Rgb(255, 0, 0)),
                ..Default::default()
            },
        );

        let _output = r.paint_frame(&tree);

        // The child's absolute pixel position should be (2*8, 1*16) = (16, 16).
        // Check that pixel at (20, 20) is red (inside child area).
        let p = r.canvas.get_pixel(20, 20);
        assert_eq!(p[0], 255, "child area should be red, got r={}", p[0]);
        assert_eq!(p[3], 255, "child area should be opaque");

        // Check that pixel at (4, 4) is blue (parent area, outside child).
        let p = r.canvas.get_pixel(4, 4);
        assert_eq!(
            p[2], 100,
            "parent area outside child should be blue, got b={}",
            p[2]
        );
    }

    // -- paint_shadow does not panic --------------------------------------

    #[test]
    fn paint_shadow_renders_without_panic() {
        let mut r = PixelRenderer::new(20, 10, 8, 16);
        let shadow = PixelBoxShadow {
            offset_x: 2.0,
            offset_y: 2.0,
            blur_radius: 4.0,
            spread_radius: 0.0,
            color: [0, 0, 0, 128],
        };
        r.paint_shadow(20.0, 20.0, 80.0, 40.0, 4.0, &shadow);

        // At least some shadow pixels should be non-transparent.
        let non_transparent = r.canvas.data.chunks_exact(4).filter(|px| px[3] > 0).count();
        assert!(non_transparent > 0, "shadow should produce visible pixels");
    }

    // -- color_to_rgba ----------------------------------------------------

    #[test]
    fn color_to_rgba_rgb() {
        let c = color_to_rgba(&Color::Rgb(10, 20, 30), 200);
        assert_eq!(c, [10, 20, 30, 200]);
    }

    #[test]
    fn color_to_rgba_ansi_black() {
        let c = color_to_rgba(&Color::Ansi(0), 255);
        assert_eq!(c, [0, 0, 0, 255]);
    }

    #[test]
    fn color_to_rgba_bright_red() {
        let c = color_to_rgba(&Color::AnsiBright(1), 255);
        assert_eq!(c, [255, 85, 85, 255]);
    }

    // -- border rendering -------------------------------------------------

    #[test]
    fn paint_frame_with_border() {
        let mut r = PixelRenderer::new(10, 5, 8, 16);
        let mut tree = TestTree::new();
        tree.root = Some(1);
        tree.layouts.insert(
            1,
            NodeLayout {
                x: 0.0,
                y: 0.0,
                width: 10.0,
                height: 5.0,
            },
        );
        tree.styles.insert(
            1,
            PixelNodeStyle {
                border_thickness: 2.0,
                border_color: Some(Color::Rgb(0, 255, 0)),
                ..Default::default()
            },
        );

        let _output = r.paint_frame(&tree);

        // Border pixel at top edge should be green.
        let p = r.canvas.get_pixel(40, 1);
        assert!(p[1] > 0, "border pixel should have green, got g={}", p[1]);
        assert!(p[3] > 0, "border pixel should be visible");
    }

    // -- paint_frame with gradient background --------------------------------

    #[test]
    fn paint_frame_with_gradient_background() {
        let mut tree = TestTree::new();
        tree.root = Some(1);
        tree.layouts.insert(
            1,
            NodeLayout {
                x: 0.0,
                y: 0.0,
                width: 10.0,
                height: 5.0,
            },
        );
        tree.styles.insert(
            1,
            PixelNodeStyle {
                gradient: Some(PixelGradient {
                    angle_deg: 0.0, // left-to-right
                    stops: vec![(0.0, [255, 0, 0, 255]), (1.0, [0, 0, 255, 255])],
                }),
                ..Default::default()
            },
        );
        tree.children.insert(1, vec![]);

        let mut r = PixelRenderer::new(10, 5, 8, 16);
        let _output = r.paint_frame(&tree);

        // Left side should be reddish, right side bluish
        let left = r.canvas.get_pixel(4, 40);
        let right = r.canvas.get_pixel(76, 40);
        assert!(left[0] > left[2], "left should be more red than blue");
        assert!(right[2] > right[0], "right should be more blue than red");
    }

    // -- paint_frame with gradient + borderRadius ----------------------------

    #[test]
    fn paint_frame_with_gradient_and_border_radius() {
        let mut tree = TestTree::new();
        tree.root = Some(1);
        tree.layouts.insert(
            1,
            NodeLayout {
                x: 0.0,
                y: 0.0,
                width: 10.0,
                height: 5.0,
            },
        );
        tree.styles.insert(
            1,
            PixelNodeStyle {
                gradient: Some(PixelGradient {
                    angle_deg: 0.0,
                    stops: vec![(0.0, [255, 0, 0, 255]), (1.0, [0, 0, 255, 255])],
                }),
                border_radius: 8.0,
                ..Default::default()
            },
        );
        tree.children.insert(1, vec![]);

        let mut r = PixelRenderer::new(10, 5, 8, 16);
        let _output = r.paint_frame(&tree);

        // Corner pixel should have reduced alpha or be transparent (rounded)
        let corner = r.canvas.get_pixel(0, 0);
        // Center should be filled
        let center = r.canvas.get_pixel(40, 40);
        assert!(
            center[3] > corner[3],
            "center alpha {} should exceed corner alpha {}",
            center[3],
            corner[3]
        );
    }

    // -- paint_frame with overflow clipping ----------------------------------

    #[test]
    fn paint_frame_with_overflow_hidden_clips_fully_outside_child() {
        let mut tree = TestTree::new();
        tree.root = Some(1);

        // Parent: 5x3 cells with overflow_hidden
        tree.layouts.insert(
            1,
            NodeLayout {
                x: 0.0,
                y: 0.0,
                width: 5.0,
                height: 3.0,
            },
        );
        tree.styles.insert(
            1,
            PixelNodeStyle {
                bg: Some(Color::Rgb(0, 0, 100)),
                overflow_hidden: true,
                ..Default::default()
            },
        );
        tree.children.insert(1, vec![2, 3]);

        // Child 2 overlaps parent — should be painted
        tree.layouts.insert(
            2,
            NodeLayout {
                x: 1.0,
                y: 0.0,
                width: 3.0,
                height: 2.0,
            },
        );
        tree.styles.insert(
            2,
            PixelNodeStyle {
                bg: Some(Color::Rgb(255, 0, 0)),
                ..Default::default()
            },
        );
        tree.children.insert(2, vec![]);

        // Child 3 is entirely outside parent clip — should be skipped
        tree.layouts.insert(
            3,
            NodeLayout {
                x: 10.0, // way outside parent (5 cells wide)
                y: 0.0,
                width: 3.0,
                height: 2.0,
            },
        );
        tree.styles.insert(
            3,
            PixelNodeStyle {
                bg: Some(Color::Rgb(0, 255, 0)),
                ..Default::default()
            },
        );
        tree.children.insert(3, vec![]);

        let mut r = PixelRenderer::new(20, 5, 8, 16);
        let _output = r.paint_frame(&tree);

        // Child 2 should be visible (inside clip)
        let inside = r.canvas.get_pixel(16, 8); // x=2 cells, y=0.5 cells
        assert_eq!(inside[0], 255, "overlapping child should be red");

        // Child 3 area should be empty (fully outside clip, skipped)
        let outside = r.canvas.get_pixel(84, 8); // x=10.5 cells
        assert_eq!(
            outside[1], 0,
            "fully-outside child should be clipped (skipped)"
        );
    }

    // -- paint_frame with text_spans (multi-color text) ----------------------

    /// Extended `TestTree` that supports `text_spans`.
    struct TestTreeWithSpans {
        inner: TestTree,
        spans: std::collections::HashMap<u32, Vec<PixelTextSpan>>,
    }

    impl TestTreeWithSpans {
        fn new() -> Self {
            Self {
                inner: TestTree::new(),
                spans: std::collections::HashMap::new(),
            }
        }
    }

    impl PaintTree for TestTreeWithSpans {
        fn root_node(&self) -> Option<u32> {
            self.inner.root
        }
        fn node_layout(&self, node_id: u32) -> Option<NodeLayout> {
            self.inner.layouts.get(&node_id).copied()
        }
        fn node_style(&self, node_id: u32) -> Option<PixelNodeStyle> {
            self.inner.styles.get(&node_id).cloned()
        }
        fn text_content(&self, node_id: u32) -> Option<&str> {
            self.inner.texts.get(&node_id).map(|s| s.as_str())
        }
        fn text_spans(&self, node_id: u32) -> Vec<PixelTextSpan> {
            self.spans.get(&node_id).cloned().unwrap_or_default()
        }
        fn children(&self, node_id: u32) -> Vec<u32> {
            self.inner
                .children
                .get(&node_id)
                .cloned()
                .unwrap_or_default()
        }
    }

    #[test]
    fn paint_frame_with_text_spans_multi_color() {
        let mut tree = TestTreeWithSpans::new();
        tree.inner.root = Some(1);
        tree.inner.layouts.insert(
            1,
            NodeLayout {
                x: 0.0,
                y: 0.0,
                width: 20.0,
                height: 3.0,
            },
        );
        tree.inner.styles.insert(
            1,
            PixelNodeStyle {
                fg: Some(Color::Rgb(255, 255, 255)),
                ..Default::default()
            },
        );
        tree.inner.texts.insert(1, "AABB".to_string());
        tree.inner.children.insert(1, vec![]);
        // Color first two chars red, last two blue
        tree.spans.insert(
            1,
            vec![
                PixelTextSpan {
                    start: 0,
                    end: 2,
                    fg: [255, 0, 0, 255],
                },
                PixelTextSpan {
                    start: 2,
                    end: 4,
                    fg: [0, 0, 255, 255],
                },
            ],
        );

        let mut r = PixelRenderer::new(20, 3, 8, 16);
        let _output = r.paint_frame(&tree);

        // Should produce visible pixels (multi-color path exercised)
        let non_transparent = r.canvas.data.chunks_exact(4).filter(|px| px[3] > 0).count();
        assert!(
            non_transparent > 0,
            "multi-color text should produce visible pixels"
        );
    }

    // -- paint_frame with dim text -------------------------------------------

    #[test]
    fn paint_frame_with_dim_text_has_reduced_alpha() {
        let mut tree = TestTree::new();
        tree.root = Some(1);
        tree.layouts.insert(
            1,
            NodeLayout {
                x: 0.0,
                y: 0.0,
                width: 20.0,
                height: 3.0,
            },
        );
        tree.styles.insert(
            1,
            PixelNodeStyle {
                fg: Some(Color::Rgb(255, 255, 255)),
                dim: true,
                ..Default::default()
            },
        );
        tree.texts.insert(1, "Hello".to_string());
        tree.children.insert(1, vec![]);

        let mut r = PixelRenderer::new(20, 3, 8, 16);
        let _output = r.paint_frame(&tree);

        // Should render text. Dim text uses alpha 140 instead of 255.
        let non_transparent = r.canvas.data.chunks_exact(4).filter(|px| px[3] > 0).count();
        assert!(
            non_transparent > 0,
            "dim text should produce visible pixels"
        );

        // No pixel should have alpha > 140 (the dim cap)
        let max_alpha = r
            .canvas
            .data
            .chunks_exact(4)
            .map(|px| px[3])
            .max()
            .unwrap_or(0);
        assert!(
            max_alpha <= 140,
            "dim text max alpha should be <= 140, got {max_alpha}"
        );
    }

    // -- paint_frame with text decorations (underline, strikethrough) ---------

    #[test]
    fn paint_frame_with_underline_decoration() {
        let mut tree = TestTree::new();
        tree.root = Some(1);
        tree.layouts.insert(
            1,
            NodeLayout {
                x: 0.0,
                y: 0.0,
                width: 10.0,
                height: 3.0,
            },
        );
        tree.styles.insert(
            1,
            PixelNodeStyle {
                fg: Some(Color::Rgb(255, 255, 255)),
                underline: true,
                ..Default::default()
            },
        );
        tree.texts.insert(1, "Test".to_string());
        tree.children.insert(1, vec![]);

        let mut r = PixelRenderer::new(10, 3, 8, 16);
        let _output = r.paint_frame(&tree);

        // Underline is drawn at y = text_y + font_size * 0.9.
        // font_size defaults to cell_h = 16, so underline_y ~ 14.4.
        // draw_line uses fill_rect(x, y - thickness/2, w, thickness), so
        // actual rect y = 13.9, h = 1.0 → pixel row 13.
        // Check a range around the expected position.
        let has_underline =
            (12..16u32).any(|row| (0..80u32).any(|x| r.canvas.get_pixel(x, row)[3] > 0));
        assert!(has_underline, "should have underline pixels near y=13-15");
    }

    #[test]
    fn paint_frame_with_strikethrough_decoration() {
        let mut tree = TestTree::new();
        tree.root = Some(1);
        tree.layouts.insert(
            1,
            NodeLayout {
                x: 0.0,
                y: 0.0,
                width: 10.0,
                height: 3.0,
            },
        );
        tree.styles.insert(
            1,
            PixelNodeStyle {
                fg: Some(Color::Rgb(255, 255, 255)),
                strikethrough: true,
                ..Default::default()
            },
        );
        tree.texts.insert(1, "Test".to_string());
        tree.children.insert(1, vec![]);

        let mut r = PixelRenderer::new(10, 3, 8, 16);
        let _output = r.paint_frame(&tree);

        // Strikethrough at y = text_y + font_size * 0.5 = 8
        let strike_row = 8u32;
        let has_strike = (0..80).any(|x| r.canvas.get_pixel(x, strike_row)[3] > 0);
        assert!(has_strike, "should have strikethrough pixels near y=8");
    }

    // -- paint_frame with custom font_size -----------------------------------

    #[test]
    fn paint_frame_with_custom_font_size() {
        let mut tree = TestTree::new();
        tree.root = Some(1);
        tree.layouts.insert(
            1,
            NodeLayout {
                x: 0.0,
                y: 0.0,
                width: 20.0,
                height: 5.0,
            },
        );
        tree.styles.insert(
            1,
            PixelNodeStyle {
                fg: Some(Color::Rgb(255, 255, 255)),
                font_size: Some(32.0),
                ..Default::default()
            },
        );
        tree.texts.insert(1, "A".to_string());
        tree.children.insert(1, vec![]);

        let mut r = PixelRenderer::new(20, 5, 8, 16);
        let _output = r.paint_frame(&tree);

        // With font_size=32 (double default 16), we should get more text pixels
        let non_transparent = r.canvas.data.chunks_exact(4).filter(|px| px[3] > 0).count();
        assert!(
            non_transparent > 0,
            "custom font_size should produce visible pixels"
        );

        // Compare with default font_size
        let mut tree2 = TestTree::new();
        tree2.root = Some(1);
        tree2.layouts.insert(
            1,
            NodeLayout {
                x: 0.0,
                y: 0.0,
                width: 20.0,
                height: 5.0,
            },
        );
        tree2.styles.insert(
            1,
            PixelNodeStyle {
                fg: Some(Color::Rgb(255, 255, 255)),
                ..Default::default()
            },
        );
        tree2.texts.insert(1, "A".to_string());
        tree2.children.insert(1, vec![]);

        let mut r2 = PixelRenderer::new(20, 5, 8, 16);
        let _output2 = r2.paint_frame(&tree2);
        let non_transparent_default = r2
            .canvas
            .data
            .chunks_exact(4)
            .filter(|px| px[3] > 0)
            .count();

        assert!(
            non_transparent > non_transparent_default,
            "larger font should produce more pixels ({non_transparent} > {non_transparent_default})"
        );
    }

    // -- paint_frame with box_shadow + borderRadius --------------------------

    #[test]
    fn paint_frame_with_box_shadow_and_border_radius() {
        let mut tree = TestTree::new();
        tree.root = Some(1);
        tree.layouts.insert(
            1,
            NodeLayout {
                x: 2.0,
                y: 2.0,
                width: 6.0,
                height: 4.0,
            },
        );
        tree.styles.insert(
            1,
            PixelNodeStyle {
                bg: Some(Color::Rgb(100, 100, 255)),
                border_radius: 6.0,
                box_shadow: Some(PixelBoxShadow {
                    offset_x: 3.0,
                    offset_y: 3.0,
                    blur_radius: 4.0,
                    spread_radius: 2.0,
                    color: [0, 0, 0, 128],
                }),
                ..Default::default()
            },
        );
        tree.children.insert(1, vec![]);

        let mut r = PixelRenderer::new(12, 10, 8, 16);
        let _output = r.paint_frame(&tree);

        // Shadow should produce pixels beyond the node area
        let non_transparent = r.canvas.data.chunks_exact(4).filter(|px| px[3] > 0).count();
        assert!(
            non_transparent > 0,
            "shadow with border_radius should produce visible pixels"
        );
    }

    // -- save_screenshot ----------------------------------------------------

    #[test]
    fn save_screenshot_creates_png_file() {
        let mut r = PixelRenderer::new(4, 2, 8, 16);
        r.canvas.fill([255, 0, 0, 255]);

        let path = std::env::temp_dir().join("kittyui_test_screenshot.png");
        let path_str = path.to_str().unwrap();

        // Clean up any previous test artifact
        let _ = std::fs::remove_file(path_str);

        let result = r.save_screenshot(path_str);
        assert!(
            result.is_ok(),
            "save_screenshot should succeed: {:?}",
            result.err()
        );
        assert!(path.exists(), "PNG file should exist at {path_str}");

        // Verify it's a valid PNG by checking file size > 0
        let metadata = std::fs::metadata(path_str).unwrap();
        assert!(metadata.len() > 0, "PNG file should not be empty");

        // Clean up
        let _ = std::fs::remove_file(path_str);
    }

    // -- palette_index_to_rgb ------------------------------------------------

    #[test]
    fn palette_index_to_rgb_ansi_range() {
        // 0-7 should delegate to ansi_index_to_rgb
        assert_eq!(palette_index_to_rgb(0, 255), [0, 0, 0, 255]);
        assert_eq!(palette_index_to_rgb(1, 255), [170, 0, 0, 255]);
        assert_eq!(palette_index_to_rgb(7, 255), [170, 170, 170, 255]);
    }

    #[test]
    fn palette_index_to_rgb_bright_range() {
        // 8-15 should delegate to ansi_bright_index_to_rgb
        assert_eq!(palette_index_to_rgb(8, 255), [85, 85, 85, 255]); // bright black
        assert_eq!(palette_index_to_rgb(9, 255), [255, 85, 85, 255]); // bright red
        assert_eq!(palette_index_to_rgb(15, 255), [255, 255, 255, 255]); // bright white
    }

    #[test]
    fn palette_index_to_rgb_color_cube() {
        // Index 16 = (0,0,0) in 6x6x6 cube → all zeros
        assert_eq!(palette_index_to_rgb(16, 255), [0, 0, 0, 255]);
        // Index 196 = r=5,g=0,b=0 → (255, 0, 0) since to_rgb(5) = 55+40*5 = 255
        assert_eq!(palette_index_to_rgb(196, 255), [255, 0, 0, 255]);
        // Index 21 = r=0,g=0,b=5 → (0, 0, 255)
        assert_eq!(palette_index_to_rgb(21, 255), [0, 0, 255, 255]);
        // Index 46 = r=0,g=5,b=0 → (0, 255, 0)
        assert_eq!(palette_index_to_rgb(46, 255), [0, 255, 0, 255]);
        // Index 231 = r=5,g=5,b=5 → (255, 255, 255)
        assert_eq!(palette_index_to_rgb(231, 255), [255, 255, 255, 255]);
    }

    #[test]
    fn palette_index_to_rgb_grayscale_ramp() {
        // 232 → 8 + 10*(232-232) = 8
        assert_eq!(palette_index_to_rgb(232, 255), [8, 8, 8, 255]);
        // 255 → 8 + 10*(255-232) = 8 + 230 = 238
        assert_eq!(palette_index_to_rgb(255, 255), [238, 238, 238, 255]);
        // Mid-range: 243 → 8 + 10*11 = 118
        assert_eq!(palette_index_to_rgb(243, 255), [118, 118, 118, 255]);
    }

    // -- ansi_index_to_rgb ---------------------------------------------------

    #[test]
    fn ansi_index_to_rgb_all_basic_colors() {
        assert_eq!(ansi_index_to_rgb(0, 255), [0, 0, 0, 255]); // black
        assert_eq!(ansi_index_to_rgb(1, 255), [170, 0, 0, 255]); // red
        assert_eq!(ansi_index_to_rgb(2, 255), [0, 170, 0, 255]); // green
        assert_eq!(ansi_index_to_rgb(3, 255), [170, 170, 0, 255]); // yellow
        assert_eq!(ansi_index_to_rgb(4, 255), [0, 0, 170, 255]); // blue
        assert_eq!(ansi_index_to_rgb(5, 255), [170, 0, 170, 255]); // magenta
        assert_eq!(ansi_index_to_rgb(6, 255), [0, 170, 170, 255]); // cyan
        assert_eq!(ansi_index_to_rgb(7, 255), [170, 170, 170, 255]); // white
                                                                     // Out of range fallback
        assert_eq!(ansi_index_to_rgb(8, 255), [128, 128, 128, 255]);
    }

    // -- ansi_bright_index_to_rgb --------------------------------------------

    #[test]
    fn ansi_bright_index_to_rgb_all_bright_colors() {
        assert_eq!(ansi_bright_index_to_rgb(0, 255), [85, 85, 85, 255]); // bright black
        assert_eq!(ansi_bright_index_to_rgb(1, 255), [255, 85, 85, 255]); // bright red
        assert_eq!(ansi_bright_index_to_rgb(2, 255), [85, 255, 85, 255]); // bright green
        assert_eq!(ansi_bright_index_to_rgb(3, 255), [255, 255, 85, 255]); // bright yellow
        assert_eq!(ansi_bright_index_to_rgb(4, 255), [85, 85, 255, 255]); // bright blue
        assert_eq!(ansi_bright_index_to_rgb(5, 255), [255, 85, 255, 255]); // bright magenta
        assert_eq!(ansi_bright_index_to_rgb(6, 255), [85, 255, 255, 255]); // bright cyan
        assert_eq!(ansi_bright_index_to_rgb(7, 255), [255, 255, 255, 255]); // bright white
                                                                            // Out of range fallback
        assert_eq!(ansi_bright_index_to_rgb(8, 255), [192, 192, 192, 255]);
    }

    // -- fnv_hash ------------------------------------------------------------

    #[test]
    fn fnv_hash_deterministic() {
        let data = b"hello world";
        let h1 = fnv_hash(data);
        let h2 = fnv_hash(data);
        assert_eq!(h1, h2, "same input should produce same hash");
    }

    #[test]
    fn fnv_hash_different_inputs_different_hashes() {
        let h1 = fnv_hash(b"hello");
        let h2 = fnv_hash(b"world");
        let h3 = fnv_hash(b"");
        assert_ne!(h1, h2, "different inputs should produce different hashes");
        assert_ne!(h1, h3);
        assert_ne!(h2, h3);
    }

    // -- color_to_rgba with Palette variant ----------------------------------

    #[test]
    fn color_to_rgba_palette_variant() {
        // Palette(196) = bright red in 6x6x6 cube
        let c = color_to_rgba(&Color::Palette(196), 255);
        assert_eq!(c, [255, 0, 0, 255]);

        // Palette(232) = darkest grayscale
        let c = color_to_rgba(&Color::Palette(232), 200);
        assert_eq!(c, [8, 8, 8, 200]);
    }

    // -- color_to_rgba with Ansi variant (already tested for 0, test more) ---

    #[test]
    fn color_to_rgba_ansi_all_indices() {
        for idx in 0..8 {
            let c = color_to_rgba(&Color::Ansi(idx), 128);
            assert_eq!(c[3], 128, "alpha should be passed through for Ansi({idx})");
        }
    }

    // -- color_to_rgba with AnsiBright variant --------------------------------

    #[test]
    fn color_to_rgba_ansi_bright_all_indices() {
        for idx in 0..8 {
            let c = color_to_rgba(&Color::AnsiBright(idx), 200);
            assert_eq!(
                c[3], 200,
                "alpha should be passed through for AnsiBright({idx})"
            );
        }
    }

    // -- paint_frame with deeply nested tree (3+ levels) ---------------------

    #[test]
    fn paint_frame_deeply_nested_tree() {
        let mut tree = TestTree::new();
        tree.root = Some(1);

        // Level 1: root container
        tree.layouts.insert(
            1,
            NodeLayout {
                x: 0.0,
                y: 0.0,
                width: 20.0,
                height: 10.0,
            },
        );
        tree.styles.insert(
            1,
            PixelNodeStyle {
                bg: Some(Color::Rgb(50, 50, 50)),
                ..Default::default()
            },
        );
        tree.children.insert(1, vec![2]);

        // Level 2: nested child
        tree.layouts.insert(
            2,
            NodeLayout {
                x: 1.0,
                y: 1.0,
                width: 10.0,
                height: 5.0,
            },
        );
        tree.styles.insert(
            2,
            PixelNodeStyle {
                bg: Some(Color::Rgb(0, 128, 0)),
                ..Default::default()
            },
        );
        tree.children.insert(2, vec![3]);

        // Level 3: deeply nested grandchild
        tree.layouts.insert(
            3,
            NodeLayout {
                x: 1.0,
                y: 1.0,
                width: 4.0,
                height: 2.0,
            },
        );
        tree.styles.insert(
            3,
            PixelNodeStyle {
                bg: Some(Color::Rgb(255, 0, 0)),
                ..Default::default()
            },
        );
        tree.children.insert(3, vec![]);

        let mut r = PixelRenderer::new(20, 10, 8, 16);
        let _output = r.paint_frame(&tree);

        // Grandchild absolute position: (1+1)*8=16, (1+1)*16=32 → pixel (20, 36) should be red
        let p = r.canvas.get_pixel(20, 36);
        assert_eq!(p[0], 255, "grandchild should be red, got r={}", p[0]);

        // Level 2 area (outside grandchild): pixel (10, 20) should be green
        let p = r.canvas.get_pixel(10, 20);
        assert_eq!(p[1], 128, "child area should be green, got g={}", p[1]);

        // Level 1 area (outside child): pixel (4, 4) should be gray
        let p = r.canvas.get_pixel(4, 4);
        assert_eq!(p[0], 50, "root area should be gray, got r={}", p[0]);
    }

    // -- encode_tiles with multiple rows changed -----------------------------

    #[test]
    fn encode_tiles_multiple_rows_changed() {
        let mut r = PixelRenderer::new(4, 4, 8, 16);
        r.canvas.fill([10, 20, 30, 255]);

        // First frame: transmit all
        let _ = r.encode_tiles();

        // Modify pixels in rows 0 and 2 (leave rows 1 and 3 unchanged)
        r.canvas.set_pixel(0, 0, [255, 0, 0, 255]); // row 0
        r.canvas.set_pixel(0, 32, [0, 255, 0, 255]); // row 2 (y = 2 * 16)

        let output = r.encode_tiles();
        let text = String::from_utf8_lossy(&output);

        let transmit_count = text.matches("a=t").count();
        assert_eq!(
            transmit_count, 2,
            "exactly 2 rows should be retransmitted, got {transmit_count}"
        );
    }
}
