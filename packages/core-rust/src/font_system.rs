//! Font subsystem powered by `fontdue` for text measurement and glyph
//! rasterization.
//!
//! [`FontSystem`] loads system sans-serif fonts (Arial on macOS, Liberation
//! Sans on Linux) and rasterizes individual glyphs.  It exposes two main
//! operations:
//!
//! - **measure** -- compute the pixel dimensions of a text run.
//! - **rasterize** -- produce positioned coverage bitmaps for compositing
//!   onto a [`crate::pixel_canvas::PixelCanvas`].
//!
//! This replaces the previous `cosmic-text` backend which hangs on macOS
//! when using any non-Monospace font family.

// Glyph positioning math uses many short variable names and numeric casts.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_lossless
)]

use fontdue::{Font, FontSettings};

/// Manages font loading, text measurement, and glyph rasterization.
pub struct FontSystem {
    sans_regular: Option<Font>,
    sans_bold: Option<Font>,
    mono_regular: Option<Font>,
}

/// System font search paths for sans-serif regular.
const SANS_REGULAR_PATHS: &[&str] = &[
    "/System/Library/Fonts/Supplemental/Arial.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    "/usr/share/fonts/TTF/LiberationSans-Regular.ttf",
    "/usr/share/fonts/TTF/DejaVuSans.ttf",
];

/// System font search paths for sans-serif bold.
const SANS_BOLD_PATHS: &[&str] = &[
    "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
    "/usr/share/fonts/TTF/LiberationSans-Bold.ttf",
    "/usr/share/fonts/TTF/DejaVuSans-Bold.ttf",
];

/// System font search paths for monospace.
const MONO_PATHS: &[&str] = &[
    "/System/Library/Fonts/Supplemental/Courier New.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
    "/usr/share/fonts/TTF/LiberationMono-Regular.ttf",
    "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
];

impl FontSystem {
    /// Create a new font system with system fonts loaded.
    #[must_use]
    pub fn new() -> Self {
        let sans_regular = Self::load_font(SANS_REGULAR_PATHS);
        let sans_bold = Self::load_font(SANS_BOLD_PATHS);
        let mono_regular = Self::load_font(MONO_PATHS);

        Self {
            sans_regular,
            sans_bold,
            mono_regular,
        }
    }

    /// Try loading a font from the first readable path in the list.
    fn load_font(paths: &[&str]) -> Option<Font> {
        for path in paths {
            if let Ok(data) = std::fs::read(path) {
                let settings = FontSettings {
                    collection_index: 0,
                    scale: 40.0,
                    ..FontSettings::default()
                };
                if let Ok(font) = Font::from_bytes(data, settings) {
                    return Some(font);
                }
            }
        }
        None
    }

    /// Pick the appropriate font for the given style.
    fn pick_font(&self, bold: bool) -> Option<&Font> {
        if bold {
            self.sans_bold
                .as_ref()
                .or(self.sans_regular.as_ref())
                .or(self.mono_regular.as_ref())
        } else {
            self.sans_regular.as_ref().or(self.mono_regular.as_ref())
        }
    }

    /// Measure the pixel dimensions of a text string.
    ///
    /// Returns `(width, height)` in pixels.  For an empty string the width
    /// is zero and the height equals the font size.
    pub fn measure_text(
        &mut self,
        text: &str,
        font_size: f32,
        bold: bool,
        _italic: bool,
    ) -> (f32, f32) {
        let Some(font) = self.pick_font(bold) else {
            return (text.len() as f32 * font_size * 0.6, font_size);
        };

        let line_metrics = font.horizontal_line_metrics(font_size);
        let line_height = line_metrics.map_or(font_size * 1.2, |lm| lm.new_line_size);

        let mut width = 0.0f32;
        for ch in text.chars() {
            let metrics = font.metrics(ch, font_size);
            width += metrics.advance_width;
        }

        (width.max(0.0), line_height)
    }

    /// Rasterize text and return positioned glyphs with their coverage
    /// bitmaps.
    ///
    /// Each [`RasterizedGlyph`] contains the bitmap position, dimensions,
    /// and an alpha-coverage buffer suitable for compositing onto a
    /// [`crate::pixel_canvas::PixelCanvas`].
    pub fn rasterize_text(
        &mut self,
        text: &str,
        font_size: f32,
        bold: bool,
        _italic: bool,
    ) -> Vec<RasterizedGlyph> {
        let Some(font) = self.pick_font(bold) else {
            return Vec::new();
        };

        // Compute ascent from the font's line metrics for proper baseline
        // positioning.
        let line_metrics = font.horizontal_line_metrics(font_size);
        let ascent = line_metrics.map_or(font_size * 0.8, |lm| lm.ascent);

        let mut glyphs = Vec::new();
        let mut x = 0.0f32;

        for ch in text.chars() {
            let (metrics, bitmap) = font.rasterize(ch, font_size);

            if metrics.width > 0 && metrics.height > 0 {
                glyphs.push(RasterizedGlyph {
                    x: x + metrics.xmin as f32,
                    y: ascent - metrics.height as f32 - metrics.ymin as f32,
                    width: metrics.width as u32,
                    height: metrics.height as u32,
                    content: GlyphContent::Mask,
                    data: bitmap,
                });
            }

            x += metrics.advance_width;
        }

        // Tighten glyph positioning: shift all glyphs up to remove the gap
        // between the font's typographic ascent and the actual top of the
        // tallest glyph.  Without this, text sits too low in its container
        // (e.g. avatar initials at the bottom of circles instead of centred).
        if !glyphs.is_empty() {
            let min_y = glyphs.iter().map(|g| g.y).fold(f32::MAX, f32::min);
            if min_y > 0.0 {
                for g in &mut glyphs {
                    g.y -= min_y;
                }
            }
        }

        glyphs
    }
}

impl Default for FontSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Describes the content format of a rasterized glyph's bitmap data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlyphContent {
    /// 8-bit alpha mask (1 byte per pixel grayscale coverage) -- for normal text.
    Mask,
    /// 32-bit RGBA bitmap (4 bytes per pixel) -- for color emoji.
    Color,
}

/// A rasterized glyph with position and coverage bitmap.
#[derive(Debug, Clone)]
pub struct RasterizedGlyph {
    /// X position in pixels (relative to text origin).
    pub x: f32,
    /// Y position in pixels (relative to text origin).
    pub y: f32,
    /// Bitmap width in pixels.
    pub width: u32,
    /// Bitmap height in pixels.
    pub height: u32,
    /// The content format of the bitmap data.
    pub content: GlyphContent,
    /// Coverage bitmap -- either grayscale (1 byte/pixel) for [`GlyphContent::Mask`]
    /// or RGBA (4 bytes/pixel) for [`GlyphContent::Color`].
    pub data: Vec<u8>,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_font_system_without_panicking() {
        let _fs = FontSystem::new();
    }

    #[test]
    fn measure_text_returns_nonzero_dimensions() {
        let mut fs = FontSystem::new();
        let (w, h) = fs.measure_text("Hello", 16.0, false, false);
        assert!(w > 0.0, "width should be > 0, got {w}");
        assert!(h > 0.0, "height should be > 0, got {h}");
    }

    #[test]
    fn rasterize_text_produces_glyphs() {
        let mut fs = FontSystem::new();
        let glyphs = fs.rasterize_text("A", 16.0, false, false);
        assert!(!glyphs.is_empty(), "should produce at least 1 glyph");
        let g = &glyphs[0];
        assert!(!g.data.is_empty(), "glyph data should not be empty");
    }

    #[test]
    fn measure_empty_string_returns_zero_width() {
        let mut fs = FontSystem::new();
        let (w, h) = fs.measure_text("", 16.0, false, false);
        assert!(
            w.abs() < f32::EPSILON,
            "empty string width should be 0, got {w}"
        );
        assert!(h >= 16.0, "height should be at least font_size, got {h}");
    }

    #[test]
    fn bold_text_has_similar_width() {
        let mut fs = FontSystem::new();
        let (w_normal, _) = fs.measure_text("Hello", 16.0, false, false);
        let (w_bold, _) = fs.measure_text("Hello", 16.0, true, false);
        // Bold and normal should have similar widths for the same text
        let diff = (w_normal - w_bold).abs();
        assert!(
            diff < w_normal * 0.25,
            "bold width ({w_bold}) should be similar to normal ({w_normal}), diff={diff}"
        );
    }

    #[test]
    fn bold_produces_different_glyph_data() {
        let mut fs = FontSystem::new();
        // Skip if bold font is not available (falls back to regular).
        if fs.sans_bold.is_none() {
            return;
        }
        let glyphs_regular = fs.rasterize_text("A", 24.0, false, false);
        let glyphs_bold = fs.rasterize_text("A", 24.0, true, false);
        assert!(!glyphs_regular.is_empty(), "regular should produce glyphs");
        assert!(!glyphs_bold.is_empty(), "bold should produce glyphs");
        // Bold glyph should have different bitmap data than regular.
        assert_ne!(
            glyphs_regular[0].data, glyphs_bold[0].data,
            "bold 'A' bitmap should differ from regular 'A' bitmap"
        );
    }

    #[test]
    fn measure_text_uses_line_metrics_height() {
        let mut fs = FontSystem::new();
        let (_, h) = fs.measure_text("Hello", 16.0, false, false);
        // Line height from metrics should be larger than the font size
        // (it includes ascent + descent + line gap).
        assert!(
            h >= 16.0,
            "line height ({h}) should be at least font_size (16.0)"
        );
    }
}
