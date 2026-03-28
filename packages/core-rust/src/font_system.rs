//! Font subsystem powered by `cosmic-text` for text measurement and glyph
//! rasterization.
//!
//! [`FontSystem`] wraps a `cosmic_text::FontSystem` (which discovers system
//! fonts automatically) and a `SwashCache` for glyph rasterization.  It
//! exposes two main operations:
//!
//! - **measure** — compute the pixel dimensions of a shaped text run.
//! - **rasterize** — produce positioned coverage bitmaps for compositing
//!   onto a [`crate::pixel_canvas::PixelCanvas`].

// Glyph positioning math uses many short variable names and numeric casts.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_lossless
)]

use cosmic_text::{
    Attrs, Buffer, Family, FontSystem as CosmicFontSystem, Metrics, Shaping, SwashCache,
    SwashContent, Weight,
};

/// Manages font loading, text measurement, and glyph rasterization.
pub struct FontSystem {
    font_system: CosmicFontSystem,
    swash_cache: SwashCache,
}

impl FontSystem {
    /// Create a new font system with system fonts loaded.
    #[must_use]
    pub fn new() -> Self {
        Self {
            font_system: CosmicFontSystem::new(),
            swash_cache: SwashCache::new(),
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
        italic: bool,
    ) -> (f32, f32) {
        let metrics = Metrics::new(font_size, font_size * 1.2);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);

        let weight = if bold { Weight::BOLD } else { Weight::NORMAL };
        let style = if italic {
            cosmic_text::Style::Italic
        } else {
            cosmic_text::Style::Normal
        };
        let attrs = Attrs::new()
            .family(Family::SansSerif)
            .weight(weight)
            .style(style);

        buffer.set_text(&mut self.font_system, text, attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut self.font_system, false);

        let width = buffer
            .layout_runs()
            .flat_map(|run| run.glyphs.iter())
            .map(|g| g.x + g.w)
            .fold(0.0f32, f32::max);
        let height = buffer.layout_runs().map(|run| run.line_height).sum::<f32>();

        (width.max(0.0), height.max(font_size))
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
        italic: bool,
    ) -> Vec<RasterizedGlyph> {
        let metrics = Metrics::new(font_size, font_size * 1.2);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);

        let weight = if bold { Weight::BOLD } else { Weight::NORMAL };
        let style = if italic {
            cosmic_text::Style::Italic
        } else {
            cosmic_text::Style::Normal
        };
        let attrs = Attrs::new()
            .family(Family::SansSerif)
            .weight(weight)
            .style(style);

        buffer.set_text(&mut self.font_system, text, attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut self.font_system, false);

        let mut glyphs = Vec::new();

        for run in buffer.layout_runs() {
            for glyph in run.glyphs {
                let physical = glyph.physical((0., 0.), 1.0);

                if let Some(image) = self
                    .swash_cache
                    .get_image(&mut self.font_system, physical.cache_key)
                {
                    let content = match image.content {
                        SwashContent::Mask => GlyphContent::Mask,
                        SwashContent::Color => GlyphContent::Color,
                        SwashContent::SubpixelMask => continue,
                    };
                    glyphs.push(RasterizedGlyph {
                        x: physical.x as f32 + image.placement.left as f32,
                        y: run.line_y + physical.y as f32 - image.placement.top as f32,
                        width: image.placement.width,
                        height: image.placement.height,
                        content,
                        data: image.data.clone(),
                    });
                }
            }
        }

        glyphs
    }

    /// Get a mutable reference to the underlying `cosmic_text::FontSystem`.
    pub fn inner_mut(&mut self) -> &mut CosmicFontSystem {
        &mut self.font_system
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
    /// 8-bit alpha mask (1 byte per pixel grayscale coverage) — for normal text.
    Mask,
    /// 32-bit RGBA bitmap (4 bytes per pixel) — for color emoji.
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
    /// Coverage bitmap — either grayscale (1 byte/pixel) for [`GlyphContent::Mask`]
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
    fn bold_has_similar_width() {
        let mut fs = FontSystem::new();
        let (w_normal, _) = fs.measure_text("Hello", 16.0, false, false);
        let (w_bold, _) = fs.measure_text("Hello", 16.0, true, false);
        // With sans-serif fonts, bold variants can be noticeably wider
        let diff = (w_normal - w_bold).abs();
        assert!(
            diff < w_normal * 0.5,
            "bold width ({w_bold}) should be within 50% of normal ({w_normal}), diff={diff}"
        );
    }
}
