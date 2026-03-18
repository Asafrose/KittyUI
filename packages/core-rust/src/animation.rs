//! Animation engine for the Kitty graphics protocol.
//!
//! Provides client-driven and terminal-driven animation, frame composition
//! with rectangular region blitting, compositing modes, and animated
//! GIF/APNG loading.

use std::io;

use ::image::codecs::gif::GifDecoder;
use ::image::codecs::png::PngDecoder as ExternalPngDecoder;
use ::image::AnimationDecoder;

use crate::image::{self, ImageCache, ImageData};

// ---------------------------------------------------------------------------
// Compositing
// ---------------------------------------------------------------------------

/// How a source region is composited onto a destination frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositingMode {
    /// Overwrite destination pixels entirely.
    Replace,
    /// Blend source over destination using source alpha.
    AlphaBlend,
}

/// Blit (copy) a rectangular region from `src` onto `dst`.
///
/// `src_rect` is `(x, y, width, height)` in the source frame.
/// `dst_pos` is `(x, y)` in the destination frame.
///
/// Pixels outside either frame are silently clipped.
pub fn blit(
    dst: &mut ImageData,
    src: &ImageData,
    src_rect: (u32, u32, u32, u32),
    dst_pos: (u32, u32),
    mode: CompositingMode,
) {
    let (sx, sy, sw, sh) = src_rect;
    let (dx, dy) = dst_pos;

    for row in 0..sh {
        let src_y = sy + row;
        let dst_y = dy + row;
        if src_y >= src.height || dst_y >= dst.height {
            continue;
        }
        for col in 0..sw {
            let src_x = sx + col;
            let dst_x = dx + col;
            if src_x >= src.width || dst_x >= dst.width {
                continue;
            }
            let si = ((src_y * src.width + src_x) * 4) as usize;
            let di = ((dst_y * dst.width + dst_x) * 4) as usize;

            match mode {
                CompositingMode::Replace => {
                    dst.rgba[di..di + 4].copy_from_slice(&src.rgba[si..si + 4]);
                }
                #[allow(clippy::cast_possible_truncation)]
                CompositingMode::AlphaBlend => {
                    let sa = u16::from(src.rgba[si + 3]);
                    let da = 255 - sa;
                    for c in 0..3 {
                        let blended = (u16::from(src.rgba[si + c]) * sa
                            + u16::from(dst.rgba[di + c]) * da)
                            / 255;
                        // blended is at most 255 since (x*sa + y*da)/255 <= 255
                        dst.rgba[di + c] = blended as u8;
                    }
                    // Output alpha: src_a + dst_a * (1 - src_a/255)
                    let out_a = sa + u16::from(dst.rgba[di + 3]) * da / 255;
                    dst.rgba[di + 3] = out_a.min(255) as u8;
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Frame
// ---------------------------------------------------------------------------

/// A single animation frame with optional per-frame timing.
#[derive(Debug, Clone)]
pub struct Frame {
    /// The image data for this frame.
    pub data: ImageData,
    /// Display duration in milliseconds. `None` means use the animation default.
    pub duration_ms: Option<u32>,
}

// ---------------------------------------------------------------------------
// AnimationState
// ---------------------------------------------------------------------------

/// Playback state of an animation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationState {
    /// Animation has not started yet.
    Stopped,
    /// Animation is actively playing.
    Playing,
    /// Animation is paused at the current frame.
    Paused,
}

// ---------------------------------------------------------------------------
// Animation
// ---------------------------------------------------------------------------

/// An animation composed of multiple frames.
///
/// Supports both client-driven (caller advances frames on a timer) and
/// terminal-driven (frames are transmitted with gap timings for the terminal
/// to cycle through) playback.
#[derive(Debug)]
pub struct Animation {
    /// Ordered list of frames.
    frames: Vec<Frame>,
    /// Index of the current frame.
    current_frame: usize,
    /// Default gap between frames in milliseconds (terminal-driven mode).
    default_gap_ms: u32,
    /// The Kitty image ID assigned to this animation.
    image_id: Option<u32>,
    /// Current playback state.
    state: AnimationState,
    /// Whether to loop the animation.
    looping: bool,
}

impl Animation {
    /// Create a new empty animation with the given default frame gap.
    #[must_use]
    pub fn new(default_gap_ms: u32) -> Self {
        Self {
            frames: Vec::new(),
            current_frame: 0,
            default_gap_ms,
            image_id: None,
            state: AnimationState::Stopped,
            looping: true,
        }
    }

    /// Set whether the animation loops.
    pub fn set_looping(&mut self, looping: bool) {
        self.looping = looping;
    }

    /// Returns whether the animation loops.
    #[must_use]
    pub fn looping(&self) -> bool {
        self.looping
    }

    /// Add a frame to the animation.
    pub fn add_frame(&mut self, frame: Frame) {
        self.frames.push(frame);
    }

    /// Returns the number of frames.
    #[must_use]
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Returns the current frame index.
    #[must_use]
    pub fn current_frame_index(&self) -> usize {
        self.current_frame
    }

    /// Returns a reference to the current frame, or `None` if there are no frames.
    #[must_use]
    pub fn current_frame(&self) -> Option<&Frame> {
        self.frames.get(self.current_frame)
    }

    /// Returns a reference to a frame by index.
    #[must_use]
    pub fn frame(&self, index: usize) -> Option<&Frame> {
        self.frames.get(index)
    }

    /// Returns a mutable reference to a frame by index.
    pub fn frame_mut(&mut self, index: usize) -> Option<&mut Frame> {
        self.frames.get_mut(index)
    }

    /// Returns the current playback state.
    #[must_use]
    pub fn state(&self) -> AnimationState {
        self.state
    }

    /// Returns the assigned Kitty image ID, if any.
    #[must_use]
    pub fn image_id(&self) -> Option<u32> {
        self.image_id
    }

    /// Returns the default gap in milliseconds.
    #[must_use]
    pub fn default_gap_ms(&self) -> u32 {
        self.default_gap_ms
    }

    // -- Client-driven animation --

    /// Set the current frame explicitly (client-driven animation).
    ///
    /// Returns `false` if the index is out of bounds.
    pub fn set_frame(&mut self, index: usize) -> bool {
        if index < self.frames.len() {
            self.current_frame = index;
            true
        } else {
            false
        }
    }

    /// Advance to the next frame. Wraps around if looping, otherwise stops
    /// at the last frame. Returns the new frame index.
    #[must_use]
    pub fn advance(&mut self) -> usize {
        if self.frames.is_empty() {
            return 0;
        }
        let next = self.current_frame + 1;
        if next >= self.frames.len() {
            if self.looping {
                self.current_frame = 0;
            }
            // else stay at last frame
        } else {
            self.current_frame = next;
        }
        self.current_frame
    }

    // -- Playback control --

    /// Start the animation from the beginning.
    pub fn start(&mut self) {
        self.current_frame = 0;
        self.state = AnimationState::Playing;
    }

    /// Stop the animation and reset to frame 0.
    pub fn stop(&mut self) {
        self.current_frame = 0;
        self.state = AnimationState::Stopped;
    }

    /// Pause the animation at the current frame.
    pub fn pause(&mut self) {
        if self.state == AnimationState::Playing {
            self.state = AnimationState::Paused;
        }
    }

    /// Resume a paused animation.
    pub fn resume(&mut self) {
        if self.state == AnimationState::Paused {
            self.state = AnimationState::Playing;
        }
    }

    // -- Kitty protocol encoding --

    /// Encode the animation for terminal-driven playback using the Kitty
    /// graphics protocol animation extension.
    ///
    /// Transmits frame 0 as the base image, then sends subsequent frames
    /// with gap timings so the terminal can cycle through them.
    ///
    /// # Errors
    ///
    /// Returns an error if encoding fails.
    pub fn encode_terminal_driven(&mut self, _cache: &mut ImageCache) -> io::Result<Vec<u8>> {
        if self.frames.is_empty() {
            return Ok(Vec::new());
        }

        let id = self.image_id.unwrap_or_else(|| {
            let new_id = ImageCache::next_id();
            self.image_id = Some(new_id);
            new_id
        });

        let mut buf = Vec::new();

        // Transmit the first frame as the base image.
        let base = &self.frames[0];
        let base_bytes = image::encode_transmit(&base.data, id)?;
        buf.extend_from_slice(&base_bytes);

        // For each subsequent frame, send an animation frame command.
        for (i, frame) in self.frames.iter().enumerate().skip(1) {
            let gap = frame.duration_ms.unwrap_or(self.default_gap_ms);
            let frame_payload = image::encode_transmit(&frame.data, id)?;

            // Animation frame header: a=f (animation frame), frame number,
            // gap in ms, and the compositing mode.
            let header = format!("\x1b_Ga=f,i={id},r={i},z={gap};",);
            buf.extend_from_slice(header.as_bytes());
            buf.extend_from_slice(b"\x1b\\");

            // Then transmit the frame data.
            buf.extend_from_slice(&frame_payload);
        }

        // Start animation: a=a,s=3 means "animate", v=1 means start looping
        let loop_val = u8::from(self.looping);
        let start_cmd = format!("\x1b_Ga=a,i={id},s=3,v={loop_val};\x1b\\");
        buf.extend_from_slice(start_cmd.as_bytes());

        self.state = AnimationState::Playing;
        Ok(buf)
    }

    /// Encode a command to display the current frame (client-driven mode).
    ///
    /// The caller is responsible for calling this on a timer and writing the
    /// result to the terminal.
    ///
    /// # Errors
    ///
    /// Returns an error if encoding fails or there are no frames.
    pub fn encode_current_frame(&self, cache: &mut ImageCache) -> io::Result<(u32, Vec<u8>)> {
        let frame = self.current_frame().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "animation has no frames")
        })?;

        let id = self.image_id.unwrap_or_else(ImageCache::next_id);
        image::transmit_image(&frame.data, None, Some(id), cache)
    }

    /// Encode a stop command for the terminal-driven animation.
    #[must_use]
    pub fn encode_stop(&self) -> Vec<u8> {
        match self.image_id {
            Some(id) => format!("\x1b_Ga=a,i={id},s=1;\x1b\\").into_bytes(),
            None => Vec::new(),
        }
    }

    /// Encode a pause command for the terminal-driven animation.
    #[must_use]
    pub fn encode_pause(&self) -> Vec<u8> {
        match self.image_id {
            Some(id) => format!("\x1b_Ga=a,i={id},s=2;\x1b\\").into_bytes(),
            None => Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Animated image loading (GIF / APNG)
// ---------------------------------------------------------------------------

/// Load an animated GIF from a file and return an [`Animation`].
///
/// Each GIF frame is converted to RGBA and added with its delay.
///
/// # Errors
///
/// Returns an error if the file cannot be read or decoded.
pub fn load_gif(path: &std::path::Path, default_gap_ms: u32) -> io::Result<Animation> {
    let file = std::fs::File::open(path)
        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, e.to_string()))?;
    let decoder = GifDecoder::new(std::io::BufReader::new(file))
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

    load_from_frames(decoder, default_gap_ms)
}

/// Load an animated PNG (APNG) from a file and return an [`Animation`].
///
/// # Errors
///
/// Returns an error if the file cannot be read or decoded.
pub fn load_apng(path: &std::path::Path, default_gap_ms: u32) -> io::Result<Animation> {
    let file = std::fs::File::open(path)
        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, e.to_string()))?;
    let png = ExternalPngDecoder::new(std::io::BufReader::new(file))
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    let decoder = png
        .apng()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

    load_from_frames(decoder, default_gap_ms)
}

/// Internal helper: extract frames from any `AnimationDecoder`.
fn load_from_frames<'a, D>(decoder: D, default_gap_ms: u32) -> io::Result<Animation>
where
    D: AnimationDecoder<'a>,
{
    let mut anim = Animation::new(default_gap_ms);

    let frames = decoder.into_frames();
    for frame_result in frames {
        let frame =
            frame_result.map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

        let (numer, denom) = frame.delay().numer_denom_ms();
        let delay_ms = if denom == 0 { 0 } else { numer / denom };

        let buf = frame.into_buffer();
        let (width, height) = buf.dimensions();
        let rgba = buf.into_raw();

        anim.add_frame(Frame {
            data: ImageData {
                rgba,
                width,
                height,
            },
            duration_ms: if delay_ms > 0 { Some(delay_ms) } else { None },
        });
    }

    Ok(anim)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_frame(width: u32, height: u32, fill: u8) -> Frame {
        let rgba = vec![fill; (width * height * 4) as usize];
        Frame {
            data: ImageData {
                rgba,
                width,
                height,
            },
            duration_ms: None,
        }
    }

    fn make_frame_with_duration(width: u32, height: u32, fill: u8, ms: u32) -> Frame {
        let rgba = vec![fill; (width * height * 4) as usize];
        Frame {
            data: ImageData {
                rgba,
                width,
                height,
            },
            duration_ms: Some(ms),
        }
    }

    // -- Animation basics --

    #[test]
    fn new_animation_is_empty() {
        let anim = Animation::new(100);
        assert_eq!(anim.frame_count(), 0);
        assert_eq!(anim.current_frame_index(), 0);
        assert_eq!(anim.state(), AnimationState::Stopped);
        assert!(anim.current_frame().is_none());
        assert!(anim.looping());
        assert_eq!(anim.default_gap_ms(), 100);
    }

    #[test]
    fn add_frames() {
        let mut anim = Animation::new(100);
        anim.add_frame(make_frame(2, 2, 0));
        anim.add_frame(make_frame(2, 2, 128));
        assert_eq!(anim.frame_count(), 2);
        assert!(anim.frame(0).is_some());
        assert!(anim.frame(1).is_some());
        assert!(anim.frame(2).is_none());
    }

    #[test]
    fn set_frame_valid() {
        let mut anim = Animation::new(100);
        anim.add_frame(make_frame(1, 1, 0));
        anim.add_frame(make_frame(1, 1, 255));
        assert!(anim.set_frame(1));
        assert_eq!(anim.current_frame_index(), 1);
    }

    #[test]
    fn set_frame_out_of_bounds() {
        let mut anim = Animation::new(100);
        anim.add_frame(make_frame(1, 1, 0));
        assert!(!anim.set_frame(5));
        assert_eq!(anim.current_frame_index(), 0);
    }

    #[test]
    fn advance_wraps_when_looping() {
        let mut anim = Animation::new(100);
        anim.add_frame(make_frame(1, 1, 0));
        anim.add_frame(make_frame(1, 1, 1));
        anim.add_frame(make_frame(1, 1, 2));

        assert_eq!(anim.advance(), 1);
        assert_eq!(anim.advance(), 2);
        assert_eq!(anim.advance(), 0); // wraps
    }

    #[test]
    fn advance_stops_at_last_when_not_looping() {
        let mut anim = Animation::new(100);
        anim.set_looping(false);
        anim.add_frame(make_frame(1, 1, 0));
        anim.add_frame(make_frame(1, 1, 1));

        assert_eq!(anim.advance(), 1);
        assert_eq!(anim.advance(), 1); // stays at last
    }

    #[test]
    fn advance_empty_animation() {
        let mut anim = Animation::new(100);
        assert_eq!(anim.advance(), 0);
    }

    // -- Playback state transitions --

    #[test]
    fn start_sets_playing() {
        let mut anim = Animation::new(100);
        anim.add_frame(make_frame(1, 1, 0));
        anim.start();
        assert_eq!(anim.state(), AnimationState::Playing);
        assert_eq!(anim.current_frame_index(), 0);
    }

    #[test]
    fn stop_resets() {
        let mut anim = Animation::new(100);
        anim.add_frame(make_frame(1, 1, 0));
        anim.add_frame(make_frame(1, 1, 1));
        anim.start();
        let _ = anim.advance();
        anim.stop();
        assert_eq!(anim.state(), AnimationState::Stopped);
        assert_eq!(anim.current_frame_index(), 0);
    }

    #[test]
    fn pause_and_resume() {
        let mut anim = Animation::new(100);
        anim.add_frame(make_frame(1, 1, 0));
        anim.start();
        assert_eq!(anim.state(), AnimationState::Playing);

        anim.pause();
        assert_eq!(anim.state(), AnimationState::Paused);

        anim.resume();
        assert_eq!(anim.state(), AnimationState::Playing);
    }

    #[test]
    fn pause_from_stopped_does_nothing() {
        let mut anim = Animation::new(100);
        anim.pause();
        assert_eq!(anim.state(), AnimationState::Stopped);
    }

    #[test]
    fn resume_from_stopped_does_nothing() {
        let mut anim = Animation::new(100);
        anim.resume();
        assert_eq!(anim.state(), AnimationState::Stopped);
    }

    // -- Blit / compositing --

    #[test]
    fn blit_replace_full() {
        let src_data = ImageData::from_rgba(vec![255; 16], 2, 2).ok().unwrap();
        let mut dst_data = ImageData::from_rgba(vec![0; 16], 2, 2).ok().unwrap();

        blit(
            &mut dst_data,
            &src_data,
            (0, 0, 2, 2),
            (0, 0),
            CompositingMode::Replace,
        );
        assert!(dst_data.rgba.iter().all(|&b| b == 255));
    }

    #[test]
    fn blit_replace_sub_region() {
        // 4x4 dst, 2x2 src, blit src(0,0,2,2) to dst(1,1)
        let src_data = ImageData::from_rgba(vec![200; 16], 2, 2).ok().unwrap();
        let mut dst_data = ImageData::from_rgba(vec![0; 64], 4, 4).ok().unwrap();

        blit(
            &mut dst_data,
            &src_data,
            (0, 0, 2, 2),
            (1, 1),
            CompositingMode::Replace,
        );

        // Check that (1,1) pixel is 200
        let idx = ((1 * 4 + 1) * 4) as usize;
        assert_eq!(dst_data.rgba[idx], 200);
        // Check that (0,0) pixel is still 0
        assert_eq!(dst_data.rgba[0], 0);
    }

    #[test]
    fn blit_clips_out_of_bounds() {
        let src_data = ImageData::from_rgba(vec![100; 16], 2, 2).ok().unwrap();
        let mut dst_data = ImageData::from_rgba(vec![0; 16], 2, 2).ok().unwrap();

        // Blit at position (1,1) — only top-left pixel of src fits
        blit(
            &mut dst_data,
            &src_data,
            (0, 0, 2, 2),
            (1, 1),
            CompositingMode::Replace,
        );

        let idx11 = ((1 * 2 + 1) * 4) as usize;
        assert_eq!(dst_data.rgba[idx11], 100);
        // (0,0) should be untouched
        assert_eq!(dst_data.rgba[0], 0);
    }

    #[test]
    fn blit_alpha_blend_opaque() {
        // Fully opaque source (alpha=255) should replace destination
        let mut src_rgba = vec![0u8; 16];
        for i in 0..4 {
            src_rgba[i * 4] = 200; // R
            src_rgba[i * 4 + 1] = 100; // G
            src_rgba[i * 4 + 2] = 50; // B
            src_rgba[i * 4 + 3] = 255; // A (fully opaque)
        }
        let src = ImageData {
            rgba: src_rgba,
            width: 2,
            height: 2,
        };
        let mut dst = ImageData::from_rgba(vec![0; 16], 2, 2).ok().unwrap();

        blit(
            &mut dst,
            &src,
            (0, 0, 2, 2),
            (0, 0),
            CompositingMode::AlphaBlend,
        );

        assert_eq!(dst.rgba[0], 200); // R
        assert_eq!(dst.rgba[1], 100); // G
        assert_eq!(dst.rgba[2], 50); // B
    }

    #[test]
    fn blit_alpha_blend_transparent() {
        // Fully transparent source (alpha=0) should leave destination unchanged
        let mut src_rgba = vec![0u8; 16];
        for i in 0..4 {
            src_rgba[i * 4] = 200;
            src_rgba[i * 4 + 1] = 100;
            src_rgba[i * 4 + 2] = 50;
            src_rgba[i * 4 + 3] = 0; // fully transparent
        }
        let src = ImageData {
            rgba: src_rgba,
            width: 2,
            height: 2,
        };
        let mut dst = ImageData::from_rgba(vec![128; 16], 2, 2).ok().unwrap();

        blit(
            &mut dst,
            &src,
            (0, 0, 2, 2),
            (0, 0),
            CompositingMode::AlphaBlend,
        );

        // Destination should be unchanged
        assert_eq!(dst.rgba[0], 128);
        assert_eq!(dst.rgba[1], 128);
        assert_eq!(dst.rgba[2], 128);
    }

    #[test]
    fn blit_alpha_blend_half() {
        // 50% alpha source
        let mut src_rgba = vec![0u8; 4];
        src_rgba[0] = 200; // R
        src_rgba[1] = 0; // G
        src_rgba[2] = 0; // B
        src_rgba[3] = 128; // ~50% alpha

        let src = ImageData {
            rgba: src_rgba,
            width: 1,
            height: 1,
        };

        let mut dst_rgba = vec![0u8; 4];
        dst_rgba[0] = 0;
        dst_rgba[1] = 200;
        dst_rgba[2] = 0;
        dst_rgba[3] = 255;

        let mut dst = ImageData {
            rgba: dst_rgba,
            width: 1,
            height: 1,
        };

        blit(
            &mut dst,
            &src,
            (0, 0, 1, 1),
            (0, 0),
            CompositingMode::AlphaBlend,
        );

        // R should be roughly halfway between 200 and 0
        assert!(dst.rgba[0] > 80 && dst.rgba[0] < 120);
        // G should be roughly halfway between 0 and 200
        assert!(dst.rgba[1] > 80 && dst.rgba[1] < 120);
    }

    // -- Protocol encoding --

    #[test]
    fn encode_terminal_driven_empty() {
        let mut anim = Animation::new(100);
        let mut cache = ImageCache::new();
        let result = anim.encode_terminal_driven(&mut cache).ok().unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn encode_terminal_driven_single_frame() {
        let mut anim = Animation::new(100);
        anim.add_frame(make_frame(2, 2, 0));
        let mut cache = ImageCache::new();
        let result = anim.encode_terminal_driven(&mut cache).ok().unwrap();
        let text = String::from_utf8(result).ok().unwrap();
        // Should contain base image transmission
        assert!(text.contains("\x1b_G"));
        assert!(text.contains("a=t"));
        // Should contain animation start command
        assert!(text.contains("a=a"));
        assert!(anim.state() == AnimationState::Playing);
    }

    #[test]
    fn encode_terminal_driven_multi_frame() {
        let mut anim = Animation::new(100);
        anim.add_frame(make_frame(2, 2, 0));
        anim.add_frame(make_frame_with_duration(2, 2, 128, 200));
        anim.add_frame(make_frame(2, 2, 255));
        let mut cache = ImageCache::new();
        let result = anim.encode_terminal_driven(&mut cache).ok().unwrap();
        let text = String::from_utf8(result).ok().unwrap();

        // Should have animation frame commands
        assert!(text.contains("a=f"));
        // Frame 1 should have gap=200
        assert!(text.contains("z=200"));
        // Frame 2 should have default gap=100
        assert!(text.contains("z=100"));
    }

    #[test]
    fn encode_terminal_driven_assigns_image_id() {
        let mut anim = Animation::new(50);
        anim.add_frame(make_frame(1, 1, 0));
        let mut cache = ImageCache::new();
        let _result = anim.encode_terminal_driven(&mut cache).ok().unwrap();
        assert!(anim.image_id().is_some());
    }

    #[test]
    fn encode_current_frame_works() {
        let mut anim = Animation::new(100);
        anim.add_frame(make_frame(2, 2, 42));
        let mut cache = ImageCache::new();
        let (id, payload) = anim.encode_current_frame(&mut cache).ok().unwrap();
        assert!(id > 0);
        assert!(!payload.is_empty());
    }

    #[test]
    fn encode_current_frame_empty_animation_errors() {
        let anim = Animation::new(100);
        let mut cache = ImageCache::new();
        let result = anim.encode_current_frame(&mut cache);
        assert!(result.is_err());
    }

    #[test]
    fn encode_stop_without_id() {
        let anim = Animation::new(100);
        assert!(anim.encode_stop().is_empty());
    }

    #[test]
    fn encode_stop_with_id() {
        let mut anim = Animation::new(100);
        anim.add_frame(make_frame(1, 1, 0));
        let mut cache = ImageCache::new();
        let _ = anim.encode_terminal_driven(&mut cache).ok().unwrap();
        let stop_bytes = anim.encode_stop();
        let text = String::from_utf8(stop_bytes).ok().unwrap();
        assert!(text.contains("s=1"));
    }

    #[test]
    fn encode_pause_with_id() {
        let mut anim = Animation::new(100);
        anim.add_frame(make_frame(1, 1, 0));
        let mut cache = ImageCache::new();
        let _ = anim.encode_terminal_driven(&mut cache).ok().unwrap();
        let pause_bytes = anim.encode_pause();
        let text = String::from_utf8(pause_bytes).ok().unwrap();
        assert!(text.contains("s=2"));
    }

    // -- Looping --

    #[test]
    fn looping_default_true() {
        let anim = Animation::new(100);
        assert!(anim.looping());
    }

    #[test]
    fn set_looping_false() {
        let mut anim = Animation::new(100);
        anim.set_looping(false);
        assert!(!anim.looping());
    }

    #[test]
    fn terminal_driven_looping_flag() {
        let mut anim = Animation::new(100);
        anim.set_looping(false);
        anim.add_frame(make_frame(1, 1, 0));
        let mut cache = ImageCache::new();
        let result = anim.encode_terminal_driven(&mut cache).ok().unwrap();
        let text = String::from_utf8(result).ok().unwrap();
        assert!(text.contains("v=0")); // looping off
    }

    // -- Frame mutation --

    #[test]
    fn frame_mut_allows_modification() {
        let mut anim = Animation::new(100);
        anim.add_frame(make_frame(2, 2, 0));
        if let Some(f) = anim.frame_mut(0) {
            f.duration_ms = Some(500);
        }
        assert_eq!(anim.frame(0).map(|f| f.duration_ms), Some(Some(500)));
    }

    // -- GIF / APNG loading (error paths) --

    #[test]
    fn load_gif_nonexistent_file() {
        let result = load_gif(std::path::Path::new("/tmp/nonexistent_kittyui.gif"), 100);
        assert!(result.is_err());
    }

    #[test]
    fn load_apng_nonexistent_file() {
        let result = load_apng(std::path::Path::new("/tmp/nonexistent_kittyui.apng"), 100);
        assert!(result.is_err());
    }

    // -- Blit edge cases --

    #[test]
    fn blit_zero_size_region() {
        let src = ImageData::from_rgba(vec![100; 16], 2, 2).ok().unwrap();
        let mut dst = ImageData::from_rgba(vec![0; 16], 2, 2).ok().unwrap();
        // Zero-width blit should not change anything
        blit(
            &mut dst,
            &src,
            (0, 0, 0, 2),
            (0, 0),
            CompositingMode::Replace,
        );
        assert!(dst.rgba.iter().all(|&b| b == 0));
    }

    #[test]
    fn blit_src_rect_partially_outside_src() {
        // src is 2x2, but we request region starting at (1,1) with size 4x4
        let src = ImageData::from_rgba(vec![200; 16], 2, 2).ok().unwrap();
        let mut dst = ImageData::from_rgba(vec![0; 64], 4, 4).ok().unwrap();
        blit(
            &mut dst,
            &src,
            (1, 1, 4, 4),
            (0, 0),
            CompositingMode::Replace,
        );
        // Only pixel (0,0) of dst should be written (from src pixel (1,1))
        assert_eq!(dst.rgba[0], 200);
        // Pixel (1,0) should still be 0 (src(2,1) is out of bounds)
        assert_eq!(dst.rgba[4], 0);
    }

    // -- Client-driven workflow integration --

    #[test]
    fn client_driven_workflow() {
        let mut anim = Animation::new(100);
        anim.add_frame(make_frame(2, 2, 10));
        anim.add_frame(make_frame(2, 2, 20));
        anim.add_frame(make_frame(2, 2, 30));

        anim.start();
        assert_eq!(anim.state(), AnimationState::Playing);
        assert_eq!(anim.current_frame_index(), 0);

        // Client advances frames explicitly
        let mut cache = ImageCache::new();
        let (_id, payload) = anim.encode_current_frame(&mut cache).ok().unwrap();
        assert!(!payload.is_empty());

        let _ = anim.advance();
        assert_eq!(anim.current_frame_index(), 1);

        anim.pause();
        assert_eq!(anim.state(), AnimationState::Paused);

        anim.resume();
        assert_eq!(anim.state(), AnimationState::Playing);

        let _ = anim.advance();
        assert_eq!(anim.current_frame_index(), 2);

        let _ = anim.advance(); // wraps
        assert_eq!(anim.current_frame_index(), 0);

        anim.stop();
        assert_eq!(anim.state(), AnimationState::Stopped);
        assert_eq!(anim.current_frame_index(), 0);
    }
}
