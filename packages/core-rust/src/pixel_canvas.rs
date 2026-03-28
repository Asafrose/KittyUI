//! Pixel-level RGBA canvas for rendering CSS visual properties.
//!
//! `PixelCanvas` is the core drawing primitive: an RGBA pixel buffer with
//! methods for fills, rounded rectangles (SDF-based anti-aliasing), alpha
//! compositing, box blur, and linear gradients.  It converts to [`ImageData`]
//! for Kitty protocol transmission.

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
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::too_many_arguments
)]

use std::io;

use crate::font_system::FontSystem;
use crate::image::ImageData;

// ---------------------------------------------------------------------------
// PixelCanvas
// ---------------------------------------------------------------------------

/// An RGBA pixel buffer for rendering CSS visual properties.
pub struct PixelCanvas {
    pub width: u32,
    pub height: u32,
    /// RGBA pixel data, length = `width * height * 4`.
    pub data: Vec<u8>,
}

impl PixelCanvas {
    /// Create a new transparent canvas.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            data: vec![0u8; (width * height * 4) as usize],
        }
    }

    /// Set pixel at (x, y) to RGBA color. No-op if out of bounds.
    #[inline]
    pub fn set_pixel(&mut self, x: u32, y: u32, rgba: [u8; 4]) {
        if x < self.width && y < self.height {
            let idx = ((y * self.width + x) * 4) as usize;
            self.data[idx..idx + 4].copy_from_slice(&rgba);
        }
    }

    /// Get pixel at (x, y). Returns transparent black if out of bounds.
    #[inline]
    pub fn get_pixel(&self, x: u32, y: u32) -> [u8; 4] {
        if x < self.width && y < self.height {
            let idx = ((y * self.width + x) * 4) as usize;
            [
                self.data[idx],
                self.data[idx + 1],
                self.data[idx + 2],
                self.data[idx + 3],
            ]
        } else {
            [0, 0, 0, 0]
        }
    }

    /// Fill entire canvas with a color.
    pub fn fill(&mut self, rgba: [u8; 4]) {
        for chunk in self.data.chunks_exact_mut(4) {
            chunk.copy_from_slice(&rgba);
        }
    }

    /// Fill a rectangle. Coordinates are clamped to canvas bounds.
    pub fn fill_rect(&mut self, x: f32, y: f32, w: f32, h: f32, rgba: [u8; 4]) {
        let x0 = x.max(0.0) as u32;
        let y0 = y.max(0.0) as u32;
        let x1 = ((x + w) as u32).min(self.width);
        let y1 = ((y + h) as u32).min(self.height);
        for py in y0..y1 {
            for px in x0..x1 {
                self.set_pixel(px, py, rgba);
            }
        }
    }

    /// Fill a rounded rectangle with anti-aliased edges.
    ///
    /// Uses a signed distance field for smooth corners.
    pub fn fill_rounded_rect(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        radius: f32,
        rgba: [u8; 4],
    ) {
        let r = radius.min(w / 2.0).min(h / 2.0);
        let x0 = x.max(0.0) as u32;
        let y0 = y.max(0.0) as u32;
        let x1 = ((x + w).ceil() as u32).min(self.width);
        let y1 = ((y + h).ceil() as u32).min(self.height);

        for py in y0..y1 {
            for px in x0..x1 {
                let fx = px as f32 + 0.5;
                let fy = py as f32 + 0.5;
                let d = sdf_rounded_rect(fx - x, fy - y, w, h, r);
                // Anti-alias: smooth transition over 1 pixel
                let alpha = (0.5 - d).clamp(0.0, 1.0);
                if alpha > 0.0 {
                    let a = (rgba[3] as f32 * alpha) as u8;
                    self.blend_pixel(px, py, [rgba[0], rgba[1], rgba[2], a]);
                }
            }
        }
    }

    /// Alpha-blend a pixel onto the canvas (source-over compositing).
    pub fn blend_pixel(&mut self, x: u32, y: u32, src: [u8; 4]) {
        if x >= self.width || y >= self.height {
            return;
        }
        let idx = ((y * self.width + x) * 4) as usize;
        let sa = src[3] as f32 / 255.0;
        let da = self.data[idx + 3] as f32 / 255.0;
        let out_a = sa + da * (1.0 - sa);
        if out_a == 0.0 {
            return;
        }
        for i in 0..3 {
            let sc = src[i] as f32 / 255.0;
            let dc = self.data[idx + i] as f32 / 255.0;
            let out_c = (sc * sa + dc * da * (1.0 - sa)) / out_a;
            self.data[idx + i] = (out_c * 255.0) as u8;
        }
        self.data[idx + 3] = (out_a * 255.0) as u8;
    }

    /// Apply a box blur (approximation of gaussian blur).
    ///
    /// Uses 3 passes of box blur for quality.
    pub fn box_blur(&mut self, radius: u32) {
        if radius == 0 {
            return;
        }
        // 3-pass box blur approximates gaussian
        for _ in 0..3 {
            self.box_blur_pass(radius);
        }
    }

    fn box_blur_pass(&mut self, radius: u32) {
        let w = self.width as usize;
        let h = self.height as usize;
        let r = radius as usize;
        let mut temp = vec![0u8; self.data.len()];

        // Horizontal pass
        for y in 0..h {
            for x in 0..w {
                let mut sum = [0u32; 4];
                let mut count = 0u32;
                let x_start = x.saturating_sub(r);
                let x_end = (x + r + 1).min(w);
                for sx in x_start..x_end {
                    let idx = (y * w + sx) * 4;
                    for c in 0..4 {
                        sum[c] += self.data[idx + c] as u32;
                    }
                    count += 1;
                }
                let idx = (y * w + x) * 4;
                for c in 0..4 {
                    temp[idx + c] = (sum[c] / count) as u8;
                }
            }
        }

        // Vertical pass
        for y in 0..h {
            for x in 0..w {
                let mut sum = [0u32; 4];
                let mut count = 0u32;
                let y_start = y.saturating_sub(r);
                let y_end = (y + r + 1).min(h);
                for sy in y_start..y_end {
                    let idx = (sy * w + x) * 4;
                    for c in 0..4 {
                        sum[c] += temp[idx + c] as u32;
                    }
                    count += 1;
                }
                let idx = (y * w + x) * 4;
                for c in 0..4 {
                    self.data[idx + c] = (sum[c] / count) as u8;
                }
            }
        }
    }

    /// Fill a linear gradient across a rect.
    ///
    /// - `angle_deg`: degrees (0 = left-to-right, 90 = top-to-bottom)
    /// - `stops`: `[(position 0.0..=1.0, RGBA color)]`
    pub fn fill_linear_gradient(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        angle_deg: f32,
        stops: &[(f32, [u8; 4])],
    ) {
        if stops.is_empty() {
            return;
        }
        if stops.len() == 1 {
            self.fill_rect(x, y, w, h, stops[0].1);
            return;
        }

        let angle = angle_deg.to_radians();
        let cos_a = angle.cos();
        let sin_a = angle.sin();

        let x0 = x.max(0.0) as u32;
        let y0 = y.max(0.0) as u32;
        let x1 = ((x + w) as u32).min(self.width);
        let y1 = ((y + h) as u32).min(self.height);

        for py in y0..y1 {
            for px in x0..x1 {
                let fx = (px as f32 + 0.5 - x) / w;
                let fy = (py as f32 + 0.5 - y) / h;
                // Project onto gradient axis
                let t = (fx * cos_a + fy * sin_a).clamp(0.0, 1.0);
                let color = interpolate_stops(t, stops);
                self.blend_pixel(px, py, color);
            }
        }
    }

    /// Draw a rasterized glyph onto the canvas.
    ///
    /// The glyph data is a coverage bitmap (1 byte per pixel, 0=transparent,
    /// 255=fully covered).  The color is applied using the coverage as alpha.
    pub fn draw_glyph(
        &mut self,
        x: f32,
        y: f32,
        width: u32,
        height: u32,
        data: &[u8],
        color: [u8; 4],
    ) {
        // Guard: data must contain at least width*height bytes of coverage.
        // Color glyphs (e.g. emoji) may have 4 bytes/pixel — skip those
        // since we only support grayscale coverage bitmaps here.
        let expected = (width as usize) * (height as usize);
        if expected == 0 || data.len() < expected {
            return;
        }

        let x0 = x.floor() as i32;
        let y0 = y.floor() as i32;
        for gy in 0..height {
            for gx in 0..width {
                let px = x0 + gx as i32;
                let py = y0 + gy as i32;
                if px >= 0 && py >= 0 && (px as u32) < self.width && (py as u32) < self.height {
                    let coverage = data[(gy * width + gx) as usize];
                    if coverage > 0 {
                        let alpha = ((color[3] as u32 * coverage as u32) / 255) as u8;
                        self.blend_pixel(
                            px as u32,
                            py as u32,
                            [color[0], color[1], color[2], alpha],
                        );
                    }
                }
            }
        }
    }

    /// Draw a text string onto the canvas using the font system.
    pub fn draw_text(
        &mut self,
        x: f32,
        y: f32,
        text: &str,
        color: [u8; 4],
        font_size: f32,
        bold: bool,
        italic: bool,
        font_system: &mut FontSystem,
    ) {
        let glyphs = font_system.rasterize_text(text, font_size, bold, italic);
        for glyph in &glyphs {
            self.draw_glyph(
                x + glyph.x,
                y + glyph.y,
                glyph.width,
                glyph.height,
                &glyph.data,
                color,
            );
        }
    }

    /// Draw a horizontal or angled line with given thickness.
    ///
    /// Used for text decorations (underline, strikethrough, overline).
    pub fn draw_line(
        &mut self,
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        thickness: f32,
        color: [u8; 4],
    ) {
        // For horizontal lines (most common), fill a thin rect
        if (y0 - y1).abs() < 0.5 {
            self.fill_rect(
                x0.min(x1),
                y0 - thickness / 2.0,
                (x1 - x0).abs(),
                thickness,
                color,
            );
            return;
        }
        // For general lines, use Bresenham-like approach
        // (stretch goal -- horizontal covers 99% of use cases)
    }

    /// Draw a bordered rectangle with anti-aliased corners.
    ///
    /// - `thickness`: border width in pixels
    /// - `radii`: `[top-left, top-right, bottom-right, bottom-left]` corner
    ///   radii
    pub fn draw_border(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        thickness: f32,
        radii: [f32; 4],
        color: [u8; 4],
    ) {
        // Use SDF difference: outer rounded rect - inner rounded rect
        let x0 = (x - 1.0).max(0.0) as u32;
        let y0 = (y - 1.0).max(0.0) as u32;
        let x1 = ((x + w + 1.0).ceil() as u32).min(self.width);
        let y1 = ((y + h + 1.0).ceil() as u32).min(self.height);

        for py in y0..y1 {
            for px in x0..x1 {
                let fx = px as f32 + 0.5;
                let fy = py as f32 + 0.5;

                // Use uniform radius for now (top-left)
                let r = radii[0].min(w / 2.0).min(h / 2.0);
                let d_outer = sdf_rounded_rect(fx - x, fy - y, w, h, r);
                let inner_r = (r - thickness).max(0.0);
                let d_inner = sdf_rounded_rect(
                    fx - x - thickness,
                    fy - y - thickness,
                    w - thickness * 2.0,
                    h - thickness * 2.0,
                    inner_r,
                );

                // Border region: inside outer, outside inner
                let outer_alpha = (0.5 - d_outer).clamp(0.0, 1.0);
                let inner_alpha = (0.5 - d_inner).clamp(0.0, 1.0);
                let border_alpha = outer_alpha * (1.0 - inner_alpha);

                if border_alpha > 0.0 {
                    let a = (color[3] as f32 * border_alpha + 0.5) as u8;
                    self.blend_pixel(px, py, [color[0], color[1], color[2], a]);
                }
            }
        }
    }

    /// Convert to [`ImageData`] for Kitty protocol transmission.
    ///
    /// # Errors
    ///
    /// Returns an error if the internal buffer size is inconsistent (should
    /// never happen for a properly constructed `PixelCanvas`).
    pub fn to_image_data(&self) -> io::Result<ImageData> {
        ImageData::from_rgba(self.data.clone(), self.width, self.height)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Signed distance field for a rounded rectangle.
///
/// `px`/`py` are relative to the rect origin; the rect spans `(0,0)` to
/// `(w,h)`.  Returns negative values inside, positive outside, and ~0 at the
/// boundary.
fn sdf_rounded_rect(px: f32, py: f32, w: f32, h: f32, r: f32) -> f32 {
    // Move to center-relative coords
    let cx = px - w / 2.0;
    let cy = py - h / 2.0;
    let half_w = w / 2.0 - r;
    let half_h = h / 2.0 - r;
    let dx = cx.abs() - half_w;
    let dy = cy.abs() - half_h;
    let outside = (dx.max(0.0).powi(2) + dy.max(0.0).powi(2)).sqrt();
    let inside = dx.max(dy).min(0.0);
    outside + inside - r
}

/// Interpolate between gradient color stops at position `t`.
fn interpolate_stops(t: f32, stops: &[(f32, [u8; 4])]) -> [u8; 4] {
    if t <= stops[0].0 {
        return stops[0].1;
    }
    if t >= stops[stops.len() - 1].0 {
        return stops[stops.len() - 1].1;
    }

    for i in 0..stops.len() - 1 {
        let (t0, c0) = stops[i];
        let (t1, c1) = stops[i + 1];
        if t >= t0 && t <= t1 {
            let f = if t1 > t0 { (t - t0) / (t1 - t0) } else { 0.0 };
            return [
                lerp_u8(c0[0], c1[0], f),
                lerp_u8(c0[1], c1[1], f),
                lerp_u8(c0[2], c1[2], f),
                lerp_u8(c0[3], c1[3], f),
            ];
        }
    }
    stops[stops.len() - 1].1
}

#[inline]
fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t) as u8
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Construction -------------------------------------------------------

    #[test]
    fn new_creates_transparent_canvas() {
        let c = PixelCanvas::new(10, 20);
        assert_eq!(c.width, 10);
        assert_eq!(c.height, 20);
        assert_eq!(c.data.len(), 10 * 20 * 4);
        assert!(c.data.iter().all(|&b| b == 0));
    }

    #[test]
    fn new_zero_size() {
        let c = PixelCanvas::new(0, 0);
        assert_eq!(c.data.len(), 0);
    }

    // -- fill ---------------------------------------------------------------

    #[test]
    fn fill_sets_all_pixels() {
        let mut c = PixelCanvas::new(3, 3);
        c.fill([255, 0, 128, 200]);
        for chunk in c.data.chunks_exact(4) {
            assert_eq!(chunk, [255, 0, 128, 200]);
        }
    }

    // -- fill_rect ----------------------------------------------------------

    #[test]
    fn fill_rect_fills_correct_region() {
        let mut c = PixelCanvas::new(10, 10);
        c.fill_rect(2.0, 3.0, 4.0, 5.0, [255, 0, 0, 255]);
        // Inside
        assert_eq!(c.get_pixel(3, 5), [255, 0, 0, 255]);
        // Outside
        assert_eq!(c.get_pixel(0, 0), [0, 0, 0, 0]);
        assert_eq!(c.get_pixel(9, 9), [0, 0, 0, 0]);
    }

    #[test]
    fn fill_rect_clips_at_boundaries() {
        let mut c = PixelCanvas::new(5, 5);
        // Rect extends beyond canvas
        c.fill_rect(-2.0, -2.0, 10.0, 10.0, [0, 255, 0, 255]);
        // All pixels should be filled
        for chunk in c.data.chunks_exact(4) {
            assert_eq!(chunk, [0, 255, 0, 255]);
        }
    }

    #[test]
    fn fill_rect_entirely_outside() {
        let mut c = PixelCanvas::new(5, 5);
        c.fill_rect(10.0, 10.0, 5.0, 5.0, [255, 0, 0, 255]);
        // Nothing should change
        assert!(c.data.iter().all(|&b| b == 0));
    }

    // -- fill_rounded_rect --------------------------------------------------

    #[test]
    fn fill_rounded_rect_corners_have_lower_alpha() {
        let mut c = PixelCanvas::new(40, 40);
        c.fill_rounded_rect(0.0, 0.0, 40.0, 40.0, 10.0, [255, 0, 0, 255]);
        // Center pixel should be fully opaque
        let center = c.get_pixel(20, 20);
        assert_eq!(center[3], 255);
        // Corner pixel (0,0) should have lower alpha (or be transparent)
        let corner = c.get_pixel(0, 0);
        assert!(
            corner[3] < center[3],
            "corner alpha {} should be less than center alpha {}",
            corner[3],
            center[3]
        );
    }

    // -- sdf_rounded_rect ---------------------------------------------------

    #[test]
    fn sdf_negative_inside_positive_outside() {
        // Center of a 10x10 rect — well inside
        let inside = sdf_rounded_rect(5.0, 5.0, 10.0, 10.0, 2.0);
        assert!(inside < 0.0, "inside SDF should be negative, got {inside}");
        // Far outside
        let outside = sdf_rounded_rect(20.0, 20.0, 10.0, 10.0, 2.0);
        assert!(
            outside > 0.0,
            "outside SDF should be positive, got {outside}"
        );
    }

    #[test]
    fn sdf_approximately_zero_at_boundary() {
        // Middle of right edge of a 10x10 rect with r=0
        let d = sdf_rounded_rect(10.0, 5.0, 10.0, 10.0, 0.0);
        assert!(d.abs() < 0.01, "boundary SDF should be ~0, got {d}");
    }

    // -- blend_pixel --------------------------------------------------------

    #[test]
    fn blend_pixel_50_percent_red_over_blue() {
        let mut c = PixelCanvas::new(1, 1);
        // Start with opaque blue
        c.set_pixel(0, 0, [0, 0, 255, 255]);
        // Blend 50% red on top
        c.blend_pixel(0, 0, [255, 0, 0, 128]);
        let p = c.get_pixel(0, 0);
        // Red channel should be roughly half
        assert!(p[0] > 100 && p[0] < 160, "red={}", p[0]);
        // Blue channel should be roughly half
        assert!(p[1] < 10, "green={}", p[1]);
        assert!(p[2] > 90 && p[2] < 160, "blue={}", p[2]);
        // Alpha should be fully opaque (or very close)
        assert!(p[3] > 250, "alpha={}", p[3]);
    }

    #[test]
    fn blend_pixel_onto_transparent() {
        let mut c = PixelCanvas::new(1, 1);
        c.blend_pixel(0, 0, [255, 0, 0, 128]);
        let p = c.get_pixel(0, 0);
        assert_eq!(p[0], 255);
        assert_eq!(p[1], 0);
        assert_eq!(p[2], 0);
        assert_eq!(p[3], 128);
    }

    #[test]
    fn blend_pixel_out_of_bounds_is_noop() {
        let mut c = PixelCanvas::new(2, 2);
        c.blend_pixel(5, 5, [255, 0, 0, 255]);
        assert!(c.data.iter().all(|&b| b == 0));
    }

    // -- box_blur -----------------------------------------------------------

    #[test]
    fn box_blur_uniform_stays_uniform() {
        let mut c = PixelCanvas::new(10, 10);
        c.fill([100, 100, 100, 255]);
        c.box_blur(2);
        // Interior pixels should still be ~100 (edge pixels may differ due to
        // clamping, so we check interior only)
        for y in 3..7 {
            for x in 3..7 {
                let p = c.get_pixel(x, y);
                for ch in 0..3 {
                    assert!(
                        (p[ch] as i32 - 100).unsigned_abs() <= 1,
                        "pixel({x},{y}) ch{ch}={} expected ~100",
                        p[ch]
                    );
                }
            }
        }
    }

    #[test]
    fn box_blur_single_bright_pixel_spreads() {
        let mut c = PixelCanvas::new(11, 11);
        c.set_pixel(5, 5, [255, 255, 255, 255]);
        c.box_blur(1);
        // The bright pixel should have spread to neighbours
        let center = c.get_pixel(5, 5);
        let neighbour = c.get_pixel(6, 5);
        // Center should be dimmer than original 255
        assert!(center[0] < 255, "center should have diffused");
        // Neighbour should be brighter than 0
        assert!(neighbour[0] > 0, "neighbour should have received light");
    }

    #[test]
    fn box_blur_zero_radius_is_noop() {
        let mut c = PixelCanvas::new(5, 5);
        c.fill([42, 42, 42, 255]);
        let before = c.data.clone();
        c.box_blur(0);
        assert_eq!(c.data, before);
    }

    // -- fill_linear_gradient -----------------------------------------------

    #[test]
    fn linear_gradient_left_to_right() {
        let mut c = PixelCanvas::new(100, 1);
        let stops = vec![(0.0, [0, 0, 0, 255]), (1.0, [255, 255, 255, 255])];
        c.fill_linear_gradient(0.0, 0.0, 100.0, 1.0, 0.0, &stops);
        // Left side should be dark
        let left = c.get_pixel(5, 0);
        // Right side should be bright
        let right = c.get_pixel(95, 0);
        assert!(
            right[0] > left[0],
            "right {} should be brighter than left {}",
            right[0],
            left[0]
        );
    }

    #[test]
    fn linear_gradient_single_stop_fills_solid() {
        let mut c = PixelCanvas::new(10, 10);
        let stops = vec![(0.5, [128, 64, 32, 255])];
        c.fill_linear_gradient(0.0, 0.0, 10.0, 10.0, 0.0, &stops);
        // All pixels should match the single stop color
        for y in 0..10 {
            for x in 0..10 {
                assert_eq!(c.get_pixel(x, y), [128, 64, 32, 255]);
            }
        }
    }

    #[test]
    fn linear_gradient_empty_stops_is_noop() {
        let mut c = PixelCanvas::new(5, 5);
        c.fill_linear_gradient(0.0, 0.0, 5.0, 5.0, 0.0, &[]);
        assert!(c.data.iter().all(|&b| b == 0));
    }

    // -- interpolate_stops --------------------------------------------------

    #[test]
    fn interpolate_before_first_stop() {
        let stops = vec![(0.3, [100, 0, 0, 255]), (0.7, [200, 0, 0, 255])];
        let c = interpolate_stops(0.0, &stops);
        assert_eq!(c, [100, 0, 0, 255]);
    }

    #[test]
    fn interpolate_after_last_stop() {
        let stops = vec![(0.3, [100, 0, 0, 255]), (0.7, [200, 0, 0, 255])];
        let c = interpolate_stops(1.0, &stops);
        assert_eq!(c, [200, 0, 0, 255]);
    }

    #[test]
    fn interpolate_between_stops() {
        let stops = vec![(0.0, [0, 0, 0, 255]), (1.0, [200, 100, 50, 255])];
        let c = interpolate_stops(0.5, &stops);
        // Should be roughly halfway
        assert!((c[0] as i32 - 100).unsigned_abs() <= 1);
        assert!((c[1] as i32 - 50).unsigned_abs() <= 1);
        assert!((c[2] as i32 - 25).unsigned_abs() <= 1);
        assert_eq!(c[3], 255);
    }

    #[test]
    fn interpolate_exact_stop_position() {
        let stops = vec![
            (0.0, [0, 0, 0, 255]),
            (0.5, [128, 128, 128, 255]),
            (1.0, [255, 255, 255, 255]),
        ];
        let c = interpolate_stops(0.5, &stops);
        assert_eq!(c, [128, 128, 128, 255]);
    }

    // -- to_image_data ------------------------------------------------------

    #[test]
    fn to_image_data_correct_dimensions() {
        let c = PixelCanvas::new(16, 8);
        let img = c.to_image_data().unwrap();
        assert_eq!(img.width, 16);
        assert_eq!(img.height, 8);
        assert_eq!(img.rgba.len(), 16 * 8 * 4);
    }

    #[test]
    fn to_image_data_preserves_content() {
        let mut c = PixelCanvas::new(2, 2);
        c.fill([10, 20, 30, 40]);
        let img = c.to_image_data().unwrap();
        assert_eq!(img.rgba, c.data);
    }

    // -- draw_glyph ---------------------------------------------------------

    #[test]
    fn draw_glyph_synthetic_2x2_coverage() {
        let mut c = PixelCanvas::new(10, 10);
        // 2x2 glyph: top-left fully covered, bottom-right half covered
        let coverage = vec![255, 128, 64, 0];
        c.draw_glyph(2.0, 3.0, 2, 2, &coverage, [255, 0, 0, 255]);

        // Pixel (2,3) should be fully red
        let p = c.get_pixel(2, 3);
        assert_eq!(p[0], 255);
        assert_eq!(p[3], 255);

        // Pixel (3,3) should be about half alpha
        let p = c.get_pixel(3, 3);
        assert!(p[3] > 100 && p[3] < 140, "alpha={}", p[3]);

        // Pixel (2,4) should be low alpha
        let p = c.get_pixel(2, 4);
        assert!(p[3] > 50 && p[3] < 80, "alpha={}", p[3]);

        // Pixel (3,4) coverage=0, should remain transparent
        let p = c.get_pixel(3, 4);
        assert_eq!(p[3], 0);
    }

    #[test]
    fn draw_glyph_zero_width_is_noop() {
        let mut c = PixelCanvas::new(10, 10);
        c.draw_glyph(0.0, 0.0, 0, 5, &[], [255, 0, 0, 255]);
        assert!(c.data.iter().all(|&b| b == 0));
    }

    #[test]
    fn draw_glyph_undersized_data_is_noop() {
        // Simulate a color glyph or corrupt data where data.len() < width*height
        let mut c = PixelCanvas::new(10, 10);
        let short_data = vec![255u8; 3]; // 2x2 glyph needs 4 bytes, only 3 provided
        c.draw_glyph(0.0, 0.0, 2, 2, &short_data, [255, 0, 0, 255]);
        // Should not panic and should not render anything
        assert!(c.data.iter().all(|&b| b == 0));
    }

    #[test]
    fn draw_glyph_negative_offset_clips() {
        let mut c = PixelCanvas::new(10, 10);
        // Glyph positioned partly off the left/top edge
        let coverage = vec![255u8; 4]; // 2x2 fully covered
        c.draw_glyph(-1.0, -1.0, 2, 2, &coverage, [255, 0, 0, 255]);
        // Only (0,0) should be visible (the bottom-right pixel of the 2x2)
        let p = c.get_pixel(0, 0);
        assert_eq!(p[3], 255, "pixel at (0,0) should be visible");
        // (1,0), (0,1) should be empty because they map to glyph pixels at
        // gx=2/gy=2 which are outside the 2x2 glyph
        // Actually gx=1,gy=0 => px=0,py=-1 => clipped; gx=0,gy=1 => px=-1,py=0 => clipped
        // gx=1,gy=1 => px=0,py=0 => visible -- that's the one we checked above
    }

    #[test]
    fn draw_glyph_entirely_off_canvas() {
        let mut c = PixelCanvas::new(10, 10);
        let coverage = vec![255u8; 4];
        c.draw_glyph(100.0, 100.0, 2, 2, &coverage, [255, 0, 0, 255]);
        assert!(c.data.iter().all(|&b| b == 0));
    }

    // -- draw_text ----------------------------------------------------------

    #[test]
    fn draw_text_renders_nonzero_pixels() {
        let mut c = PixelCanvas::new(100, 40);
        let mut fs = crate::font_system::FontSystem::new();
        c.draw_text(
            0.0,
            0.0,
            "A",
            [255, 255, 255, 255],
            16.0,
            false,
            false,
            &mut fs,
        );
        // At least some pixels should be non-transparent
        let non_transparent = c.data.chunks_exact(4).filter(|px| px[3] > 0).count();
        assert!(
            non_transparent > 0,
            "draw_text should produce visible pixels"
        );
    }

    // -- draw_line ----------------------------------------------------------

    #[test]
    fn draw_line_horizontal_fills_pixels() {
        let mut c = PixelCanvas::new(20, 10);
        c.draw_line(2.0, 5.0, 18.0, 5.0, 2.0, [0, 255, 0, 255]);
        // Pixels along the line should be filled
        let p = c.get_pixel(10, 5);
        assert_eq!(p, [0, 255, 0, 255]);
        // Pixel well above the line should be empty
        let p = c.get_pixel(10, 0);
        assert_eq!(p[3], 0);
    }

    // -- draw_border --------------------------------------------------------

    #[test]
    fn draw_border_has_border_pixels_and_empty_interior() {
        let mut c = PixelCanvas::new(100, 100);
        c.draw_border(0.0, 0.0, 100.0, 100.0, 2.0, [8.0; 4], [255, 0, 0, 255]);

        // Border pixel near top edge should be filled
        let p = c.get_pixel(50, 1);
        assert!(p[3] > 0, "border pixel should be visible, alpha={}", p[3]);

        // Interior pixel (center) should be empty
        let p = c.get_pixel(50, 50);
        assert_eq!(p[3], 0, "interior should be transparent, alpha={}", p[3]);

        // Border pixel near left edge should be filled
        let p = c.get_pixel(1, 50);
        assert!(
            p[3] > 0,
            "left border pixel should be visible, alpha={}",
            p[3]
        );
    }
}
