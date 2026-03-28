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

        // Centre glyphs vertically within the line height.
        // Without this, text sits at the typographic baseline position which
        // leaves a gap at the top and descender space at the bottom.
        if !glyphs.is_empty() {
            let min_y = glyphs.iter().map(|g| g.y).fold(f32::MAX, f32::min);
            let max_y = glyphs
                .iter()
                .map(|g| g.y + g.height as f32)
                .fold(0.0f32, f32::max);
            let text_height = max_y - min_y;
            let line_height = line_metrics.map_or(font_size, |lm| lm.new_line_size);
            let offset = (line_height - text_height) / 2.0 - min_y;
            for g in &mut glyphs {
                g.y += offset;
            }
        }

        glyphs
    }
}

/// Test-only constructor for a `FontSystem` with no fonts loaded.
#[cfg(test)]
impl FontSystem {
    pub fn new_empty() -> Self {
        Self {
            sans_regular: None,
            sans_bold: None,
            mono_regular: None,
        }
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

    // -- measure_text with bold=true -----------------------------------------

    #[test]
    fn measure_text_bold_returns_nonzero() {
        let mut fs = FontSystem::new();
        let (w, h) = fs.measure_text("Hello", 16.0, true, false);
        assert!(w > 0.0, "bold width should be > 0, got {w}");
        assert!(h > 0.0, "bold height should be > 0, got {h}");
    }

    // -- measure_text with italic=true ---------------------------------------

    #[test]
    fn measure_text_italic_returns_nonzero() {
        let mut fs = FontSystem::new();
        let (w, h) = fs.measure_text("Hello", 16.0, false, true);
        assert!(w > 0.0, "italic width should be > 0, got {w}");
        assert!(h > 0.0, "italic height should be > 0, got {h}");
    }

    // -- rasterize_text with bold=true ---------------------------------------

    #[test]
    fn rasterize_text_bold_produces_glyphs() {
        let mut fs = FontSystem::new();
        let glyphs = fs.rasterize_text("A", 16.0, true, false);
        assert!(
            !glyphs.is_empty(),
            "bold rasterize should produce at least 1 glyph"
        );
        let g = &glyphs[0];
        assert!(!g.data.is_empty(), "bold glyph data should not be empty");
        assert_eq!(g.content, GlyphContent::Mask, "should be a mask glyph");
    }

    // -- rasterize_text with special characters ------------------------------

    #[test]
    fn rasterize_text_digits_and_punctuation() {
        let mut fs = FontSystem::new();
        let glyphs = fs.rasterize_text("123!@#", 16.0, false, false);
        assert!(
            !glyphs.is_empty(),
            "digits and punctuation should produce glyphs"
        );
        // Each visible character should produce a glyph
        assert!(
            glyphs.len() >= 3,
            "should have at least 3 glyphs for '123!@#', got {}",
            glyphs.len()
        );
    }

    // -- font fallback (sans → mono) -----------------------------------------

    #[test]
    fn pick_font_falls_back_to_mono() {
        // We can't easily remove sans fonts at runtime, but we can verify
        // that pick_font returns Some for both bold and non-bold on this
        // system (it should always find at least one font).
        let fs = FontSystem::new();
        let regular = fs.pick_font(false);
        assert!(
            regular.is_some(),
            "pick_font(false) should find at least one font"
        );
        let bold = fs.pick_font(true);
        assert!(
            bold.is_some(),
            "pick_font(true) should find at least one font (with fallback)"
        );
    }

    // -- rasterize_text empty string -----------------------------------------

    #[test]
    fn rasterize_text_empty_string_returns_empty() {
        let mut fs = FontSystem::new();
        let glyphs = fs.rasterize_text("", 16.0, false, false);
        assert!(glyphs.is_empty(), "empty string should produce no glyphs");
    }

    // -- measure_text scales with font size ----------------------------------

    #[test]
    fn measure_text_larger_font_wider() {
        let mut fs = FontSystem::new();
        let (w_small, _) = fs.measure_text("Hello", 12.0, false, false);
        let (w_large, _) = fs.measure_text("Hello", 24.0, false, false);
        assert!(
            w_large > w_small,
            "larger font ({w_large}) should be wider than smaller ({w_small})"
        );
    }

    // -- rasterize_text glyph positions are tightened -----------------------

    #[test]
    fn rasterize_text_glyphs_centered_in_line_height() {
        let mut fs = FontSystem::new();
        let glyphs = fs.rasterize_text("Ag", 24.0, false, false);
        assert!(!glyphs.is_empty());
        let min_y = glyphs.iter().map(|g| g.y).fold(f32::MAX, f32::min);
        let max_y = glyphs
            .iter()
            .map(|g| g.y + g.height as f32)
            .fold(0.0f32, f32::max);
        // Glyphs should be centered: roughly equal space above and below
        // min_y > 0 means there's top padding, max_y < line_height means bottom padding
        assert!(
            min_y >= 0.0,
            "glyphs should have non-negative y, got {min_y}"
        );
        assert!(
            max_y <= 30.0, // line height for 24px font is ~28-30px
            "glyphs should fit within line height, got max_y={max_y}"
        );
    }

    // -- Default trait -------------------------------------------------------

    #[test]
    fn font_system_default_does_not_panic() {
        let _fs = FontSystem::default();
    }

    // -- FontSystem::new_empty (no fonts loaded) ----------------------------

    #[test]
    fn pick_font_empty_returns_none() {
        let fs = FontSystem::new_empty();
        assert!(fs.pick_font(false).is_none());
        assert!(fs.pick_font(true).is_none());
    }

    #[test]
    fn measure_text_no_fonts_returns_fallback() {
        let mut fs = FontSystem::new_empty();
        let (w, h) = fs.measure_text("Hello", 16.0, false, false);
        // Fallback: width = len * font_size * 0.6 = 5 * 16.0 * 0.6 = 48.0
        assert!((w - 48.0).abs() < f32::EPSILON, "expected 48.0, got {w}");
        assert!((h - 16.0).abs() < f32::EPSILON, "expected 16.0, got {h}");
    }

    #[test]
    fn rasterize_text_no_fonts_returns_empty() {
        let mut fs = FontSystem::new_empty();
        let glyphs = fs.rasterize_text("Hello", 16.0, false, false);
        assert!(glyphs.is_empty());
    }
}
