//! Mouse input parsing for SGR and SGR-pixel mouse protocols.
//!
//! Parses escape sequences into structured [`MouseEvent`] values.
//! Supports press, release, move, and scroll events with modifier tracking.
//!
//! # SGR mouse protocol
//!
//! ```text
//! CSI < Cb ; Cx ; Cy M   — button press / motion
//! CSI < Cb ; Cx ; Cy m   — button release
//! ```
//!
//! The `Cb` (button) byte encodes the button and modifiers as a bitmask.
//!
//! # SGR-pixel mouse protocol
//!
//! Same format but coordinates are in pixels rather than cells.
//! We parse both and let the caller supply cell dimensions for conversion.

use std::fmt;
use std::io::{self, Write};

// -----------------------------------------------------------------------
// Enable / disable escape sequences
// -----------------------------------------------------------------------

/// ANSI sequence to enable basic mouse tracking (button events).
const ENABLE_MOUSE_TRACKING: &[u8] = b"\x1b[?1000h";

/// ANSI sequence to enable SGR extended mouse mode.
const ENABLE_SGR_MOUSE: &[u8] = b"\x1b[?1006h";

/// ANSI sequence to enable SGR-pixel mouse mode (Kitty extension).
const ENABLE_SGR_PIXEL_MOUSE: &[u8] = b"\x1b[?1016h";

/// ANSI sequence to enable mouse motion tracking (move events while button held).
#[allow(dead_code)]
const ENABLE_MOUSE_MOTION: &[u8] = b"\x1b[?1002h";

/// ANSI sequence to enable all mouse motion (move events even without button).
const ENABLE_ALL_MOTION: &[u8] = b"\x1b[?1003h";

/// Disable basic mouse tracking.
const DISABLE_MOUSE_TRACKING: &[u8] = b"\x1b[?1000l";

/// Disable SGR extended mouse mode.
const DISABLE_SGR_MOUSE: &[u8] = b"\x1b[?1006l";

/// Disable SGR-pixel mouse mode.
const DISABLE_SGR_PIXEL_MOUSE: &[u8] = b"\x1b[?1016l";

/// Disable mouse motion tracking.
const DISABLE_MOUSE_MOTION: &[u8] = b"\x1b[?1002l";

/// Disable all mouse motion.
const DISABLE_ALL_MOTION: &[u8] = b"\x1b[?1003l";

/// Write the escape sequences to enable full Kitty pixel-precise mouse mode.
///
/// # Errors
///
/// Returns an error if writing fails.
pub fn enable<W: Write>(w: &mut W) -> io::Result<()> {
    w.write_all(ENABLE_MOUSE_TRACKING)?;
    w.write_all(ENABLE_SGR_MOUSE)?;
    w.write_all(ENABLE_SGR_PIXEL_MOUSE)?;
    w.write_all(ENABLE_ALL_MOTION)?;
    w.flush()
}

/// Write the escape sequences to disable mouse mode.
///
/// # Errors
///
/// Returns an error if writing fails.
pub fn disable<W: Write>(w: &mut W) -> io::Result<()> {
    w.write_all(DISABLE_ALL_MOTION)?;
    w.write_all(DISABLE_MOUSE_MOTION)?;
    w.write_all(DISABLE_SGR_PIXEL_MOUSE)?;
    w.write_all(DISABLE_SGR_MOUSE)?;
    w.write_all(DISABLE_MOUSE_TRACKING)?;
    w.flush()
}

// -----------------------------------------------------------------------
// Button
// -----------------------------------------------------------------------

/// Mouse button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Left,
    Middle,
    Right,
    /// Back / side button (button 4 in X11 numbering, after scroll).
    Back,
    /// Forward / side button (button 5 in X11 numbering).
    Forward,
    /// No specific button (used for motion-only events).
    None,
}

impl fmt::Display for Button {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Left => write!(f, "Left"),
            Self::Middle => write!(f, "Middle"),
            Self::Right => write!(f, "Right"),
            Self::Back => write!(f, "Back"),
            Self::Forward => write!(f, "Forward"),
            Self::None => write!(f, "None"),
        }
    }
}

// -----------------------------------------------------------------------
// Mouse event type
// -----------------------------------------------------------------------

/// The kind of mouse event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseEventType {
    Press,
    Release,
    Move,
    ScrollUp,
    ScrollDown,
    ScrollLeft,
    ScrollRight,
}

impl fmt::Display for MouseEventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Press => write!(f, "Press"),
            Self::Release => write!(f, "Release"),
            Self::Move => write!(f, "Move"),
            Self::ScrollUp => write!(f, "ScrollUp"),
            Self::ScrollDown => write!(f, "ScrollDown"),
            Self::ScrollLeft => write!(f, "ScrollLeft"),
            Self::ScrollRight => write!(f, "ScrollRight"),
        }
    }
}

// -----------------------------------------------------------------------
// Modifiers (reuse the same bitmask layout as keyboard)
// -----------------------------------------------------------------------

/// Mouse event modifiers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    bits: u8,
}

#[allow(dead_code)]
impl Modifiers {
    pub const SHIFT: Self = Self { bits: 0b0001 };
    pub const ALT: Self = Self { bits: 0b0010 };
    pub const CTRL: Self = Self { bits: 0b0100 };

    #[must_use]
    pub const fn empty() -> Self {
        Self { bits: 0 }
    }

    #[must_use]
    pub const fn from_bits(bits: u8) -> Self {
        Self { bits }
    }

    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        (self.bits & other.bits) == other.bits
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.bits == 0
    }

    #[must_use]
    pub const fn bits(self) -> u8 {
        self.bits
    }
}

impl core::ops::BitOr for Modifiers {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self {
            bits: self.bits | rhs.bits,
        }
    }
}

// -----------------------------------------------------------------------
// MouseEvent
// -----------------------------------------------------------------------

/// A fully parsed mouse event.
#[derive(Clone, Debug, PartialEq)]
pub struct MouseEvent {
    /// What kind of event.
    pub event_type: MouseEventType,
    /// Which button (if applicable).
    pub button: Button,
    /// Cell column (1-based from terminal, converted to 0-based).
    pub x: u16,
    /// Cell row (1-based from terminal, converted to 0-based).
    pub y: u16,
    /// Pixel X coordinate (0 if not available).
    pub pixel_x: u16,
    /// Pixel Y coordinate (0 if not available).
    pub pixel_y: u16,
    /// Active modifiers.
    pub modifiers: Modifiers,
}

impl MouseEvent {
    /// Fill in pixel coordinates from cell coordinates and cell dimensions.
    #[must_use]
    pub fn with_pixel_coords(mut self, cell_width: u16, cell_height: u16) -> Self {
        if self.pixel_x == 0 && self.pixel_y == 0 && cell_width > 0 && cell_height > 0 {
            self.pixel_x = self.x * cell_width;
            self.pixel_y = self.y * cell_height;
        }
        self
    }

    /// Fill in cell coordinates from pixel coordinates and cell dimensions.
    #[must_use]
    pub fn with_cell_coords(mut self, cell_width: u16, cell_height: u16) -> Self {
        if cell_width > 0 && cell_height > 0 {
            self.x = self.pixel_x / cell_width;
            self.y = self.pixel_y / cell_height;
        }
        self
    }
}

// -----------------------------------------------------------------------
// Parser
// -----------------------------------------------------------------------

/// Parse error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseError {
    /// Input is empty.
    Empty,
    /// Not an SGR mouse sequence.
    NotMouseSequence,
    /// Malformed parameters.
    MalformedParams,
    /// Incomplete sequence (need more bytes).
    Incomplete,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "empty input"),
            Self::NotMouseSequence => write!(f, "not a mouse escape sequence"),
            Self::MalformedParams => write!(f, "malformed SGR mouse parameters"),
            Self::Incomplete => write!(f, "incomplete sequence"),
        }
    }
}

/// Parse an SGR or SGR-pixel mouse event from a byte slice.
///
/// Returns the parsed event and the number of bytes consumed.
///
/// # Errors
///
/// Returns a [`ParseError`] if the input cannot be parsed.
pub fn parse(input: &[u8]) -> Result<(MouseEvent, usize), ParseError> {
    if input.is_empty() {
        return Err(ParseError::Empty);
    }

    // SGR mouse: ESC [ < Cb ; Cx ; Cy M/m
    if input.len() < 3 {
        return Err(ParseError::Incomplete);
    }
    if input[0] != 0x1b || input[1] != b'[' || input[2] != b'<' {
        return Err(ParseError::NotMouseSequence);
    }

    // Find the terminator: 'M' (press/move) or 'm' (release).
    let start = 3;
    let mut end = start;
    while end < input.len() {
        if input[end] == b'M' || input[end] == b'm' {
            break;
        }
        end += 1;
    }
    if end >= input.len() {
        return Err(ParseError::Incomplete);
    }

    let is_release = input[end] == b'm';
    let consumed = end + 1;

    let params_str =
        std::str::from_utf8(&input[start..end]).map_err(|_| ParseError::MalformedParams)?;
    let parts: Vec<&str> = params_str.split(';').collect();
    if parts.len() != 3 {
        return Err(ParseError::MalformedParams);
    }

    let cb: u16 = parts[0].parse().map_err(|_| ParseError::MalformedParams)?;
    let cx: u16 = parts[1].parse().map_err(|_| ParseError::MalformedParams)?;
    let cy: u16 = parts[2].parse().map_err(|_| ParseError::MalformedParams)?;

    // Decode modifiers from Cb bits 2..3 (shift=4, alt=8, ctrl=16).
    let mod_bits = ((cb >> 2) & 0b0111) as u8;
    let modifiers = Modifiers::from_bits(mod_bits);

    // Decode button and event type from Cb.
    //
    // Bit layout of Cb:
    //   0-1: button (0=left, 1=middle, 2=right, 3=none/release)
    //   2:   shift
    //   3:   alt
    //   4:   ctrl
    //   5:   motion
    //   6:   scroll (bits 0-1 give direction)
    //   7:   extra buttons (128=back, 129=forward, etc.)
    let is_motion = cb & 32 != 0;
    let is_scroll = cb & 64 != 0 && cb & 128 == 0;
    let is_extra_button = cb & 128 != 0;
    let button_low = cb & 0b11; // bits 0-1

    let (event_type, button) = if is_scroll {
        let scroll_type = match button_low {
            1 => MouseEventType::ScrollDown,
            2 => MouseEventType::ScrollLeft,
            3 => MouseEventType::ScrollRight,
            _ => MouseEventType::ScrollUp,
        };
        (scroll_type, Button::None)
    } else if is_release {
        let btn = decode_button(button_low, is_extra_button);
        (MouseEventType::Release, btn)
    } else if is_motion {
        let btn = if button_low == 3 && !is_extra_button {
            Button::None
        } else {
            decode_button(button_low, is_extra_button)
        };
        (MouseEventType::Move, btn)
    } else {
        let btn = decode_button(button_low, is_extra_button);
        (MouseEventType::Press, btn)
    };

    // SGR uses 1-based coordinates; convert to 0-based.
    let x = cx.saturating_sub(1);
    let y = cy.saturating_sub(1);

    Ok((
        MouseEvent {
            event_type,
            button,
            x,
            y,
            pixel_x: 0,
            pixel_y: 0,
            modifiers,
        },
        consumed,
    ))
}

/// Parse an SGR-pixel mouse event.
///
/// Same format as SGR but coordinates are in pixels.
/// The caller should provide cell dimensions to derive cell coordinates.
///
/// # Errors
///
/// Returns a [`ParseError`] if the input cannot be parsed.
pub fn parse_pixel(
    input: &[u8],
    cell_width: u16,
    cell_height: u16,
) -> Result<(MouseEvent, usize), ParseError> {
    let (mut event, consumed) = parse(input)?;
    // In pixel mode, the x/y we parsed are actually pixel coords.
    // Shift them to pixel fields and compute cell coords.
    event.pixel_x = event.x;
    event.pixel_y = event.y;
    if cell_width > 0 && cell_height > 0 {
        event.x = event.pixel_x / cell_width;
        event.y = event.pixel_y / cell_height;
    } else {
        event.x = 0;
        event.y = 0;
    }
    Ok((event, consumed))
}

/// Decode button from the low 2 bits of Cb and whether it's an extra button.
fn decode_button(low: u16, is_extra: bool) -> Button {
    if is_extra {
        match low {
            0 => Button::Back,
            1 => Button::Forward,
            _ => Button::None,
        }
    } else {
        match low {
            0 => Button::Left,
            1 => Button::Middle,
            2 => Button::Right,
            _ => Button::None,
        }
    }
}

// -----------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Enable / disable ---

    #[test]
    fn enable_writes_sequences() {
        let mut buf = Vec::new();
        enable(&mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("\x1b[?1000h"));
        assert!(s.contains("\x1b[?1006h"));
        assert!(s.contains("\x1b[?1016h"));
        assert!(s.contains("\x1b[?1003h"));
    }

    #[test]
    fn disable_writes_sequences() {
        let mut buf = Vec::new();
        disable(&mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("\x1b[?1000l"));
        assert!(s.contains("\x1b[?1006l"));
        assert!(s.contains("\x1b[?1016l"));
        assert!(s.contains("\x1b[?1003l"));
    }

    // --- Left click ---

    #[test]
    fn parse_left_press() {
        // CSI < 0 ; 10 ; 5 M
        let (ev, len) = parse(b"\x1b[<0;10;5M").unwrap();
        assert_eq!(ev.event_type, MouseEventType::Press);
        assert_eq!(ev.button, Button::Left);
        assert_eq!(ev.x, 9); // 10 - 1
        assert_eq!(ev.y, 4); // 5 - 1
        assert!(ev.modifiers.is_empty());
        assert_eq!(len, 10);
    }

    #[test]
    fn parse_left_release() {
        // CSI < 0 ; 10 ; 5 m
        let (ev, _) = parse(b"\x1b[<0;10;5m").unwrap();
        assert_eq!(ev.event_type, MouseEventType::Release);
        assert_eq!(ev.button, Button::Left);
    }

    // --- Right click ---

    #[test]
    fn parse_right_press() {
        // Cb = 2 → right button
        let (ev, _) = parse(b"\x1b[<2;1;1M").unwrap();
        assert_eq!(ev.event_type, MouseEventType::Press);
        assert_eq!(ev.button, Button::Right);
    }

    #[test]
    fn parse_right_release() {
        let (ev, _) = parse(b"\x1b[<2;1;1m").unwrap();
        assert_eq!(ev.event_type, MouseEventType::Release);
        assert_eq!(ev.button, Button::Right);
    }

    // --- Middle click ---

    #[test]
    fn parse_middle_press() {
        // Cb = 1 → middle button
        let (ev, _) = parse(b"\x1b[<1;1;1M").unwrap();
        assert_eq!(ev.event_type, MouseEventType::Press);
        assert_eq!(ev.button, Button::Middle);
    }

    // --- Back / forward buttons ---

    #[test]
    fn parse_back_button() {
        // Cb = 128 (bit 7 set, low bits 0) → back
        let (ev, _) = parse(b"\x1b[<128;1;1M").unwrap();
        assert_eq!(ev.button, Button::Back);
    }

    #[test]
    fn parse_forward_button() {
        // Cb = 129 (bit 7 set, low bit 1) → forward
        let (ev, _) = parse(b"\x1b[<129;1;1M").unwrap();
        assert_eq!(ev.button, Button::Forward);
    }

    // --- Scroll ---

    #[test]
    fn parse_scroll_up() {
        // Cb = 64 → scroll up
        let (ev, _) = parse(b"\x1b[<64;1;1M").unwrap();
        assert_eq!(ev.event_type, MouseEventType::ScrollUp);
        assert_eq!(ev.button, Button::None);
    }

    #[test]
    fn parse_scroll_down() {
        // Cb = 65 → scroll down
        let (ev, _) = parse(b"\x1b[<65;1;1M").unwrap();
        assert_eq!(ev.event_type, MouseEventType::ScrollDown);
    }

    #[test]
    fn parse_scroll_left() {
        // Cb = 66 → scroll left
        let (ev, _) = parse(b"\x1b[<66;1;1M").unwrap();
        assert_eq!(ev.event_type, MouseEventType::ScrollLeft);
    }

    #[test]
    fn parse_scroll_right() {
        // Cb = 67 → scroll right
        let (ev, _) = parse(b"\x1b[<67;1;1M").unwrap();
        assert_eq!(ev.event_type, MouseEventType::ScrollRight);
    }

    // --- Motion ---

    #[test]
    fn parse_motion_with_left_held() {
        // Cb = 32 (motion bit) + 0 (left) = 32
        let (ev, _) = parse(b"\x1b[<32;20;10M").unwrap();
        assert_eq!(ev.event_type, MouseEventType::Move);
        assert_eq!(ev.button, Button::Left);
        assert_eq!(ev.x, 19);
        assert_eq!(ev.y, 9);
    }

    #[test]
    fn parse_motion_no_button() {
        // Cb = 35 (motion bit 32 + button bits 3 = no button)
        let (ev, _) = parse(b"\x1b[<35;5;5M").unwrap();
        assert_eq!(ev.event_type, MouseEventType::Move);
        assert_eq!(ev.button, Button::None);
    }

    // --- Modifiers ---

    #[test]
    fn parse_shift_click() {
        // Cb = 4 (shift bit) + 0 (left) = 4
        let (ev, _) = parse(b"\x1b[<4;1;1M").unwrap();
        assert_eq!(ev.event_type, MouseEventType::Press);
        assert_eq!(ev.button, Button::Left);
        assert!(ev.modifiers.contains(Modifiers::SHIFT));
        assert!(!ev.modifiers.contains(Modifiers::ALT));
        assert!(!ev.modifiers.contains(Modifiers::CTRL));
    }

    #[test]
    fn parse_alt_click() {
        // Cb = 8 (alt bit) + 0 (left) = 8
        let (ev, _) = parse(b"\x1b[<8;1;1M").unwrap();
        assert!(ev.modifiers.contains(Modifiers::ALT));
    }

    #[test]
    fn parse_ctrl_click() {
        // Cb = 16 (ctrl bit) + 0 (left) = 16
        let (ev, _) = parse(b"\x1b[<16;1;1M").unwrap();
        assert!(ev.modifiers.contains(Modifiers::CTRL));
    }

    #[test]
    fn parse_ctrl_shift_click() {
        // Cb = 4 (shift) + 16 (ctrl) + 0 (left) = 20
        let (ev, _) = parse(b"\x1b[<20;1;1M").unwrap();
        assert!(ev.modifiers.contains(Modifiers::SHIFT));
        assert!(ev.modifiers.contains(Modifiers::CTRL));
    }

    #[test]
    fn parse_all_modifiers_click() {
        // Cb = 4 (shift) + 8 (alt) + 16 (ctrl) = 28
        let (ev, _) = parse(b"\x1b[<28;1;1M").unwrap();
        assert!(ev.modifiers.contains(Modifiers::SHIFT));
        assert!(ev.modifiers.contains(Modifiers::ALT));
        assert!(ev.modifiers.contains(Modifiers::CTRL));
    }

    // --- Coordinates ---

    #[test]
    fn parse_large_coordinates() {
        let (ev, _) = parse(b"\x1b[<0;200;100M").unwrap();
        assert_eq!(ev.x, 199);
        assert_eq!(ev.y, 99);
    }

    #[test]
    fn parse_coordinate_one_is_zero_based() {
        let (ev, _) = parse(b"\x1b[<0;1;1M").unwrap();
        assert_eq!(ev.x, 0);
        assert_eq!(ev.y, 0);
    }

    // --- SGR-pixel mode ---

    #[test]
    fn parse_pixel_mode() {
        // Pixel coordinates: x=80, y=160 with 8px wide, 16px tall cells
        let (ev, _) = parse_pixel(b"\x1b[<0;81;161M", 8, 16).unwrap();
        assert_eq!(ev.pixel_x, 80); // 81 - 1
        assert_eq!(ev.pixel_y, 160); // 161 - 1
        assert_eq!(ev.x, 10); // 80 / 8
        assert_eq!(ev.y, 10); // 160 / 16
    }

    #[test]
    fn parse_pixel_mode_fractional_cell() {
        // Pixel 12, 25 with 8x16 cells → cell 1, 1 (integer division)
        let (ev, _) = parse_pixel(b"\x1b[<0;13;26M", 8, 16).unwrap();
        assert_eq!(ev.pixel_x, 12);
        assert_eq!(ev.pixel_y, 25);
        assert_eq!(ev.x, 1); // 12 / 8
        assert_eq!(ev.y, 1); // 25 / 16
    }

    // --- with_pixel_coords / with_cell_coords ---

    #[test]
    fn with_pixel_coords_fills_pixels() {
        let ev = MouseEvent {
            event_type: MouseEventType::Press,
            button: Button::Left,
            x: 10,
            y: 5,
            pixel_x: 0,
            pixel_y: 0,
            modifiers: Modifiers::empty(),
        }
        .with_pixel_coords(8, 16);

        assert_eq!(ev.pixel_x, 80);
        assert_eq!(ev.pixel_y, 80);
    }

    #[test]
    fn with_cell_coords_fills_cells() {
        let ev = MouseEvent {
            event_type: MouseEventType::Press,
            button: Button::Left,
            x: 0,
            y: 0,
            pixel_x: 80,
            pixel_y: 160,
            modifiers: Modifiers::empty(),
        }
        .with_cell_coords(8, 16);

        assert_eq!(ev.x, 10);
        assert_eq!(ev.y, 10);
    }

    // --- Error cases ---

    #[test]
    fn parse_empty() {
        assert_eq!(parse(b""), Err(ParseError::Empty));
    }

    #[test]
    fn parse_not_mouse() {
        assert_eq!(parse(b"\x1b[A"), Err(ParseError::NotMouseSequence));
    }

    #[test]
    fn parse_incomplete() {
        assert_eq!(parse(b"\x1b[<0;1;1"), Err(ParseError::Incomplete));
    }

    #[test]
    fn parse_malformed() {
        assert_eq!(parse(b"\x1b[<abc;1;1M"), Err(ParseError::MalformedParams));
    }

    #[test]
    fn parse_wrong_param_count() {
        assert_eq!(parse(b"\x1b[<0;1M"), Err(ParseError::MalformedParams));
    }

    // --- Consumed bytes ---

    #[test]
    fn consumed_bytes_short_coords() {
        let (_, len) = parse(b"\x1b[<0;1;1M").unwrap();
        assert_eq!(len, 9);
    }

    #[test]
    fn consumed_bytes_long_coords() {
        let (_, len) = parse(b"\x1b[<0;200;100Mrest").unwrap();
        assert_eq!(len, 13);
    }

    // --- Display ---

    #[test]
    fn button_display() {
        assert_eq!(Button::Left.to_string(), "Left");
        assert_eq!(Button::Right.to_string(), "Right");
        assert_eq!(Button::Middle.to_string(), "Middle");
        assert_eq!(Button::Back.to_string(), "Back");
        assert_eq!(Button::Forward.to_string(), "Forward");
        assert_eq!(Button::None.to_string(), "None");
    }

    #[test]
    fn event_type_display() {
        assert_eq!(MouseEventType::Press.to_string(), "Press");
        assert_eq!(MouseEventType::Release.to_string(), "Release");
        assert_eq!(MouseEventType::Move.to_string(), "Move");
        assert_eq!(MouseEventType::ScrollUp.to_string(), "ScrollUp");
    }

    // --- Modifiers struct ---

    #[test]
    fn modifiers_empty_is_empty() {
        assert!(Modifiers::empty().is_empty());
    }

    #[test]
    fn modifiers_combine() {
        let m = Modifiers::CTRL | Modifiers::SHIFT;
        assert!(m.contains(Modifiers::CTRL));
        assert!(m.contains(Modifiers::SHIFT));
        assert!(!m.contains(Modifiers::ALT));
    }

    // --- Modifier + scroll ---

    #[test]
    fn parse_ctrl_scroll_up() {
        // Cb = 64 (scroll) + 16 (ctrl) = 80
        let (ev, _) = parse(b"\x1b[<80;1;1M").unwrap();
        assert_eq!(ev.event_type, MouseEventType::ScrollUp);
        assert!(ev.modifiers.contains(Modifiers::CTRL));
    }

    // --- Modifier + motion ---

    #[test]
    fn parse_shift_motion() {
        // Cb = 32 (motion) + 4 (shift) + 0 (left) = 36
        let (ev, _) = parse(b"\x1b[<36;1;1M").unwrap();
        assert_eq!(ev.event_type, MouseEventType::Move);
        assert!(ev.modifiers.contains(Modifiers::SHIFT));
    }

    // ===================================================================
    // 1. Malformed / incomplete SGR sequences
    // ===================================================================

    #[test]
    fn incomplete_just_esc() {
        assert_eq!(parse(b"\x1b"), Err(ParseError::Incomplete));
    }

    #[test]
    fn incomplete_esc_bracket() {
        assert_eq!(parse(b"\x1b["), Err(ParseError::Incomplete));
    }

    #[test]
    fn incomplete_esc_bracket_lt() {
        assert_eq!(parse(b"\x1b[<"), Err(ParseError::Incomplete));
    }

    #[test]
    fn incomplete_params_no_terminator() {
        assert_eq!(parse(b"\x1b[<0;10;5"), Err(ParseError::Incomplete));
    }

    #[test]
    fn malformed_non_numeric_x() {
        assert_eq!(parse(b"\x1b[<0;abc;1M"), Err(ParseError::MalformedParams));
    }

    #[test]
    fn malformed_non_numeric_y() {
        assert_eq!(parse(b"\x1b[<0;1;xyzM"), Err(ParseError::MalformedParams));
    }

    #[test]
    fn malformed_non_numeric_cb() {
        assert_eq!(parse(b"\x1b[<xx;1;1M"), Err(ParseError::MalformedParams));
    }

    #[test]
    fn malformed_too_many_params() {
        assert_eq!(parse(b"\x1b[<0;1;1;1M"), Err(ParseError::MalformedParams));
    }

    #[test]
    fn malformed_single_param() {
        assert_eq!(parse(b"\x1b[<0M"), Err(ParseError::MalformedParams));
    }

    #[test]
    fn malformed_empty_params() {
        assert_eq!(parse(b"\x1b[<M"), Err(ParseError::MalformedParams));
    }

    #[test]
    fn not_mouse_plain_csi() {
        assert_eq!(parse(b"\x1b[1;2H"), Err(ParseError::NotMouseSequence));
    }

    #[test]
    fn not_mouse_plain_text() {
        assert_eq!(parse(b"hello"), Err(ParseError::NotMouseSequence));
    }

    #[test]
    fn malformed_negative_params() {
        // Negative numbers won't parse as u16.
        assert_eq!(parse(b"\x1b[<-1;1;1M"), Err(ParseError::MalformedParams));
    }

    #[test]
    fn malformed_float_params() {
        assert_eq!(parse(b"\x1b[<0;1.5;1M"), Err(ParseError::MalformedParams));
    }

    // ===================================================================
    // 2. Boundary coordinates
    // ===================================================================

    #[test]
    fn coords_origin_one_one() {
        let (ev, _) = parse(b"\x1b[<0;1;1M").unwrap();
        assert_eq!(ev.x, 0);
        assert_eq!(ev.y, 0);
    }

    #[test]
    fn coords_origin_zero_saturates() {
        // Coordinate 0 → saturating_sub(1) = 0.
        let (ev, _) = parse(b"\x1b[<0;0;0M").unwrap();
        assert_eq!(ev.x, 0);
        assert_eq!(ev.y, 0);
    }

    #[test]
    fn coords_large_terminal() {
        // 500 columns, 200 rows.
        let (ev, _) = parse(b"\x1b[<0;500;200M").unwrap();
        assert_eq!(ev.x, 499);
        assert_eq!(ev.y, 199);
    }

    #[test]
    fn coords_max_u16() {
        let (ev, _) = parse(b"\x1b[<0;65535;65535M").unwrap();
        assert_eq!(ev.x, 65534);
        assert_eq!(ev.y, 65534);
    }

    #[test]
    fn coords_single_digit() {
        let (ev, _) = parse(b"\x1b[<0;2;3M").unwrap();
        assert_eq!(ev.x, 1);
        assert_eq!(ev.y, 2);
    }

    // ===================================================================
    // 3. All button + modifier combinations
    // ===================================================================

    #[test]
    fn left_with_shift() {
        // Cb = 0 (left) + 4 (shift) = 4
        let (ev, _) = parse(b"\x1b[<4;1;1M").unwrap();
        assert_eq!(ev.button, Button::Left);
        assert!(ev.modifiers.contains(Modifiers::SHIFT));
        assert!(!ev.modifiers.contains(Modifiers::ALT));
        assert!(!ev.modifiers.contains(Modifiers::CTRL));
    }

    #[test]
    fn left_with_alt() {
        // Cb = 0 + 8 = 8
        let (ev, _) = parse(b"\x1b[<8;1;1M").unwrap();
        assert_eq!(ev.button, Button::Left);
        assert!(ev.modifiers.contains(Modifiers::ALT));
    }

    #[test]
    fn left_with_ctrl() {
        // Cb = 0 + 16 = 16
        let (ev, _) = parse(b"\x1b[<16;1;1M").unwrap();
        assert_eq!(ev.button, Button::Left);
        assert!(ev.modifiers.contains(Modifiers::CTRL));
    }

    #[test]
    fn right_with_shift() {
        // Cb = 2 + 4 = 6
        let (ev, _) = parse(b"\x1b[<6;1;1M").unwrap();
        assert_eq!(ev.button, Button::Right);
        assert!(ev.modifiers.contains(Modifiers::SHIFT));
    }

    #[test]
    fn right_with_alt() {
        // Cb = 2 + 8 = 10
        let (ev, _) = parse(b"\x1b[<10;1;1M").unwrap();
        assert_eq!(ev.button, Button::Right);
        assert!(ev.modifiers.contains(Modifiers::ALT));
    }

    #[test]
    fn right_with_ctrl() {
        // Cb = 2 + 16 = 18
        let (ev, _) = parse(b"\x1b[<18;1;1M").unwrap();
        assert_eq!(ev.button, Button::Right);
        assert!(ev.modifiers.contains(Modifiers::CTRL));
    }

    #[test]
    fn middle_with_shift() {
        // Cb = 1 + 4 = 5
        let (ev, _) = parse(b"\x1b[<5;1;1M").unwrap();
        assert_eq!(ev.button, Button::Middle);
        assert!(ev.modifiers.contains(Modifiers::SHIFT));
    }

    #[test]
    fn middle_with_all_modifiers() {
        // Cb = 1 (middle) + 4 (shift) + 8 (alt) + 16 (ctrl) = 29
        let (ev, _) = parse(b"\x1b[<29;1;1M").unwrap();
        assert_eq!(ev.button, Button::Middle);
        assert!(ev.modifiers.contains(Modifiers::SHIFT));
        assert!(ev.modifiers.contains(Modifiers::ALT));
        assert!(ev.modifiers.contains(Modifiers::CTRL));
    }

    #[test]
    fn left_with_shift_alt() {
        // Cb = 0 + 4 + 8 = 12
        let (ev, _) = parse(b"\x1b[<12;1;1M").unwrap();
        assert_eq!(ev.button, Button::Left);
        assert!(ev.modifiers.contains(Modifiers::SHIFT));
        assert!(ev.modifiers.contains(Modifiers::ALT));
        assert!(!ev.modifiers.contains(Modifiers::CTRL));
    }

    #[test]
    fn left_with_alt_ctrl() {
        // Cb = 0 + 8 + 16 = 24
        let (ev, _) = parse(b"\x1b[<24;1;1M").unwrap();
        assert_eq!(ev.button, Button::Left);
        assert!(ev.modifiers.contains(Modifiers::ALT));
        assert!(ev.modifiers.contains(Modifiers::CTRL));
    }

    #[test]
    fn back_button_with_ctrl() {
        // Cb = 128 (extra) + 16 (ctrl) = 144
        let (ev, _) = parse(b"\x1b[<144;1;1M").unwrap();
        assert_eq!(ev.button, Button::Back);
        assert!(ev.modifiers.contains(Modifiers::CTRL));
    }

    #[test]
    fn forward_button_with_shift() {
        // Cb = 129 (extra+fwd) + 4 (shift) = 133
        let (ev, _) = parse(b"\x1b[<133;1;1M").unwrap();
        assert_eq!(ev.button, Button::Forward);
        assert!(ev.modifiers.contains(Modifiers::SHIFT));
    }

    // ===================================================================
    // 4. Scroll events with modifiers
    // ===================================================================

    #[test]
    fn scroll_up_no_modifiers() {
        let (ev, _) = parse(b"\x1b[<64;10;5M").unwrap();
        assert_eq!(ev.event_type, MouseEventType::ScrollUp);
        assert!(ev.modifiers.is_empty());
        assert_eq!(ev.x, 9);
        assert_eq!(ev.y, 4);
    }

    #[test]
    fn scroll_down_with_shift() {
        // Cb = 65 (scroll down) + 4 (shift) = 69
        let (ev, _) = parse(b"\x1b[<69;1;1M").unwrap();
        assert_eq!(ev.event_type, MouseEventType::ScrollDown);
        assert!(ev.modifiers.contains(Modifiers::SHIFT));
    }

    #[test]
    fn scroll_up_with_ctrl() {
        // Cb = 64 + 16 = 80
        let (ev, _) = parse(b"\x1b[<80;1;1M").unwrap();
        assert_eq!(ev.event_type, MouseEventType::ScrollUp);
        assert!(ev.modifiers.contains(Modifiers::CTRL));
    }

    #[test]
    fn scroll_up_with_alt() {
        // Cb = 64 + 8 = 72
        let (ev, _) = parse(b"\x1b[<72;1;1M").unwrap();
        assert_eq!(ev.event_type, MouseEventType::ScrollUp);
        assert!(ev.modifiers.contains(Modifiers::ALT));
    }

    #[test]
    fn scroll_down_with_all_modifiers() {
        // Cb = 65 + 4 + 8 + 16 = 93
        let (ev, _) = parse(b"\x1b[<93;1;1M").unwrap();
        assert_eq!(ev.event_type, MouseEventType::ScrollDown);
        assert!(ev.modifiers.contains(Modifiers::SHIFT));
        assert!(ev.modifiers.contains(Modifiers::ALT));
        assert!(ev.modifiers.contains(Modifiers::CTRL));
    }

    #[test]
    fn scroll_left_with_ctrl() {
        // Cb = 66 + 16 = 82
        let (ev, _) = parse(b"\x1b[<82;1;1M").unwrap();
        assert_eq!(ev.event_type, MouseEventType::ScrollLeft);
        assert!(ev.modifiers.contains(Modifiers::CTRL));
    }

    #[test]
    fn scroll_right_with_alt() {
        // Cb = 67 + 8 = 75
        let (ev, _) = parse(b"\x1b[<75;1;1M").unwrap();
        assert_eq!(ev.event_type, MouseEventType::ScrollRight);
        assert!(ev.modifiers.contains(Modifiers::ALT));
    }

    // ===================================================================
    // 5. Move events with and without buttons held
    // ===================================================================

    #[test]
    fn motion_with_right_held() {
        // Cb = 32 (motion) + 2 (right) = 34
        let (ev, _) = parse(b"\x1b[<34;10;10M").unwrap();
        assert_eq!(ev.event_type, MouseEventType::Move);
        assert_eq!(ev.button, Button::Right);
    }

    #[test]
    fn motion_with_middle_held() {
        // Cb = 32 + 1 = 33
        let (ev, _) = parse(b"\x1b[<33;10;10M").unwrap();
        assert_eq!(ev.event_type, MouseEventType::Move);
        assert_eq!(ev.button, Button::Middle);
    }

    #[test]
    fn motion_no_button_explicit() {
        // Cb = 35 (motion + button bits 3 = none)
        let (ev, _) = parse(b"\x1b[<35;50;25M").unwrap();
        assert_eq!(ev.event_type, MouseEventType::Move);
        assert_eq!(ev.button, Button::None);
        assert_eq!(ev.x, 49);
        assert_eq!(ev.y, 24);
    }

    #[test]
    fn motion_with_left_and_shift() {
        // Cb = 32 (motion) + 0 (left) + 4 (shift) = 36
        let (ev, _) = parse(b"\x1b[<36;1;1M").unwrap();
        assert_eq!(ev.event_type, MouseEventType::Move);
        assert_eq!(ev.button, Button::Left);
        assert!(ev.modifiers.contains(Modifiers::SHIFT));
    }

    #[test]
    fn motion_with_right_and_ctrl() {
        // Cb = 32 + 2 + 16 = 50
        let (ev, _) = parse(b"\x1b[<50;1;1M").unwrap();
        assert_eq!(ev.event_type, MouseEventType::Move);
        assert_eq!(ev.button, Button::Right);
        assert!(ev.modifiers.contains(Modifiers::CTRL));
    }

    #[test]
    fn motion_with_all_modifiers_no_button() {
        // Cb = 32 + 3 (no btn) + 4 + 8 + 16 = 63
        let (ev, _) = parse(b"\x1b[<63;1;1M").unwrap();
        assert_eq!(ev.event_type, MouseEventType::Move);
        assert_eq!(ev.button, Button::None);
        assert!(ev.modifiers.contains(Modifiers::SHIFT));
        assert!(ev.modifiers.contains(Modifiers::ALT));
        assert!(ev.modifiers.contains(Modifiers::CTRL));
    }

    // ===================================================================
    // 6. Rapid sequential mouse events in one buffer
    // ===================================================================

    #[test]
    fn sequential_press_then_release() {
        let input = b"\x1b[<0;10;5M\x1b[<0;10;5m";
        let (ev1, l1) = parse(input).unwrap();
        assert_eq!(ev1.event_type, MouseEventType::Press);
        assert_eq!(ev1.button, Button::Left);
        let (ev2, _) = parse(&input[l1..]).unwrap();
        assert_eq!(ev2.event_type, MouseEventType::Release);
        assert_eq!(ev2.button, Button::Left);
    }

    #[test]
    fn sequential_three_events() {
        let input = b"\x1b[<0;1;1M\x1b[<32;2;2M\x1b[<0;3;3m";
        let (ev1, l1) = parse(input).unwrap();
        assert_eq!(ev1.event_type, MouseEventType::Press);

        let (ev2, l2) = parse(&input[l1..]).unwrap();
        assert_eq!(ev2.event_type, MouseEventType::Move);
        assert_eq!(ev2.x, 1);

        let (ev3, _) = parse(&input[l1 + l2..]).unwrap();
        assert_eq!(ev3.event_type, MouseEventType::Release);
        assert_eq!(ev3.x, 2);
    }

    #[test]
    fn sequential_scroll_burst() {
        let input = b"\x1b[<64;10;10M\x1b[<64;10;10M\x1b[<64;10;10M";
        let (ev1, l1) = parse(input).unwrap();
        assert_eq!(ev1.event_type, MouseEventType::ScrollUp);
        let (ev2, l2) = parse(&input[l1..]).unwrap();
        assert_eq!(ev2.event_type, MouseEventType::ScrollUp);
        let (ev3, _) = parse(&input[l1 + l2..]).unwrap();
        assert_eq!(ev3.event_type, MouseEventType::ScrollUp);
    }

    #[test]
    fn sequential_mixed_buttons() {
        let input = b"\x1b[<0;1;1M\x1b[<2;5;5M\x1b[<1;10;10M";
        let (ev1, l1) = parse(input).unwrap();
        assert_eq!(ev1.button, Button::Left);
        let (ev2, l2) = parse(&input[l1..]).unwrap();
        assert_eq!(ev2.button, Button::Right);
        let (ev3, _) = parse(&input[l1 + l2..]).unwrap();
        assert_eq!(ev3.button, Button::Middle);
    }

    #[test]
    fn sequential_with_trailing_data() {
        let input = b"\x1b[<0;1;1Mgarbage";
        let (ev, len) = parse(input).unwrap();
        assert_eq!(ev.event_type, MouseEventType::Press);
        assert_eq!(len, 9);
        // Remaining bytes should be "garbage".
        assert_eq!(&input[len..], b"garbage");
    }

    // ===================================================================
    // 7. SGR vs SGR-pixel format differences
    // ===================================================================

    #[test]
    fn sgr_cell_coordinates() {
        let (ev, _) = parse(b"\x1b[<0;80;24M").unwrap();
        assert_eq!(ev.x, 79);
        assert_eq!(ev.y, 23);
        assert_eq!(ev.pixel_x, 0);
        assert_eq!(ev.pixel_y, 0);
    }

    #[test]
    fn sgr_pixel_coordinates() {
        let (ev, _) = parse_pixel(b"\x1b[<0;640;384M", 8, 16).unwrap();
        assert_eq!(ev.pixel_x, 639);
        assert_eq!(ev.pixel_y, 383);
        assert_eq!(ev.x, 79); // 639 / 8
        assert_eq!(ev.y, 23); // 383 / 16
    }

    #[test]
    fn sgr_pixel_zero_cell_size() {
        // Zero cell size → cell coords become 0.
        let (ev, _) = parse_pixel(b"\x1b[<0;100;100M", 0, 0).unwrap();
        assert_eq!(ev.pixel_x, 99);
        assert_eq!(ev.pixel_y, 99);
        assert_eq!(ev.x, 0);
        assert_eq!(ev.y, 0);
    }

    #[test]
    fn sgr_pixel_small_cell_size() {
        let (ev, _) = parse_pixel(b"\x1b[<0;17;33M", 8, 16).unwrap();
        assert_eq!(ev.pixel_x, 16);
        assert_eq!(ev.pixel_y, 32);
        assert_eq!(ev.x, 2);
        assert_eq!(ev.y, 2);
    }

    #[test]
    fn sgr_pixel_subpixel_rounding() {
        // Integer division truncates: 7 / 8 = 0.
        let (ev, _) = parse_pixel(b"\x1b[<0;8;17M", 8, 16).unwrap();
        assert_eq!(ev.pixel_x, 7); // 8-1
        assert_eq!(ev.pixel_y, 16); // 17-1
        assert_eq!(ev.x, 0); // 7 / 8 = 0
        assert_eq!(ev.y, 1); // 16 / 16 = 1
    }

    #[test]
    fn sgr_pixel_large_coords() {
        // Large pixel coords: 8000 x 4000.
        let (ev, _) = parse_pixel(b"\x1b[<0;8001;4001M", 8, 16).unwrap();
        assert_eq!(ev.pixel_x, 8000);
        assert_eq!(ev.pixel_y, 4000);
        assert_eq!(ev.x, 1000);
        assert_eq!(ev.y, 250);
    }

    #[test]
    fn sgr_pixel_release() {
        let (ev, _) = parse_pixel(b"\x1b[<0;81;161m", 8, 16).unwrap();
        assert_eq!(ev.event_type, MouseEventType::Release);
        assert_eq!(ev.pixel_x, 80);
        assert_eq!(ev.pixel_y, 160);
    }

    #[test]
    fn sgr_pixel_scroll() {
        let (ev, _) = parse_pixel(b"\x1b[<64;81;161M", 8, 16).unwrap();
        assert_eq!(ev.event_type, MouseEventType::ScrollUp);
        assert_eq!(ev.pixel_x, 80);
        assert_eq!(ev.pixel_y, 160);
    }

    // ===================================================================
    // 8. Edge cases
    // ===================================================================

    #[test]
    fn release_without_prior_press() {
        // Release is valid on its own (terminal doesn't track state).
        let (ev, _) = parse(b"\x1b[<0;10;5m").unwrap();
        assert_eq!(ev.event_type, MouseEventType::Release);
        assert_eq!(ev.button, Button::Left);
    }

    #[test]
    fn release_right_button() {
        let (ev, _) = parse(b"\x1b[<2;1;1m").unwrap();
        assert_eq!(ev.event_type, MouseEventType::Release);
        assert_eq!(ev.button, Button::Right);
    }

    #[test]
    fn release_middle_button() {
        let (ev, _) = parse(b"\x1b[<1;1;1m").unwrap();
        assert_eq!(ev.event_type, MouseEventType::Release);
        assert_eq!(ev.button, Button::Middle);
    }

    #[test]
    fn move_at_terminal_origin() {
        let (ev, _) = parse(b"\x1b[<35;1;1M").unwrap();
        assert_eq!(ev.event_type, MouseEventType::Move);
        assert_eq!(ev.x, 0);
        assert_eq!(ev.y, 0);
    }

    #[test]
    fn move_at_terminal_far_corner() {
        let (ev, _) = parse(b"\x1b[<35;300;100M").unwrap();
        assert_eq!(ev.event_type, MouseEventType::Move);
        assert_eq!(ev.x, 299);
        assert_eq!(ev.y, 99);
    }

    #[test]
    fn with_pixel_coords_noop_when_already_set() {
        let ev = MouseEvent {
            event_type: MouseEventType::Press,
            button: Button::Left,
            x: 10,
            y: 5,
            pixel_x: 80,
            pixel_y: 80,
            modifiers: Modifiers::empty(),
        }
        .with_pixel_coords(8, 16);
        // Should NOT overwrite already-set pixel coords.
        assert_eq!(ev.pixel_x, 80);
        assert_eq!(ev.pixel_y, 80);
    }

    #[test]
    fn with_pixel_coords_zero_cell_size() {
        let ev = MouseEvent {
            event_type: MouseEventType::Press,
            button: Button::Left,
            x: 10,
            y: 5,
            pixel_x: 0,
            pixel_y: 0,
            modifiers: Modifiers::empty(),
        }
        .with_pixel_coords(0, 0);
        // Zero cell size → no conversion.
        assert_eq!(ev.pixel_x, 0);
        assert_eq!(ev.pixel_y, 0);
    }

    #[test]
    fn with_cell_coords_zero_cell_size() {
        let ev = MouseEvent {
            event_type: MouseEventType::Press,
            button: Button::Left,
            x: 0,
            y: 0,
            pixel_x: 100,
            pixel_y: 200,
            modifiers: Modifiers::empty(),
        }
        .with_cell_coords(0, 0);
        // Zero cell size → x and y stay 0.
        assert_eq!(ev.x, 0);
        assert_eq!(ev.y, 0);
    }

    #[test]
    fn extra_button_none_for_unknown_low_bits() {
        // Cb = 130 (extra, low bits 2 → None for extra buttons)
        let (ev, _) = parse(b"\x1b[<130;1;1M").unwrap();
        assert_eq!(ev.button, Button::None);
    }

    #[test]
    fn extra_button_three_is_none() {
        // Cb = 131 (extra, low bits 3 → None)
        let (ev, _) = parse(b"\x1b[<131;1;1M").unwrap();
        assert_eq!(ev.button, Button::None);
    }

    // ===================================================================
    // Additional: ParseError display coverage
    // ===================================================================

    #[test]
    fn parse_error_display() {
        assert_eq!(ParseError::Empty.to_string(), "empty input");
        assert_eq!(
            ParseError::NotMouseSequence.to_string(),
            "not a mouse escape sequence"
        );
        assert_eq!(
            ParseError::MalformedParams.to_string(),
            "malformed SGR mouse parameters"
        );
        assert_eq!(ParseError::Incomplete.to_string(), "incomplete sequence");
    }

    // ===================================================================
    // Additional: MouseEventType display full coverage
    // ===================================================================

    #[test]
    fn event_type_display_full() {
        assert_eq!(MouseEventType::Press.to_string(), "Press");
        assert_eq!(MouseEventType::Release.to_string(), "Release");
        assert_eq!(MouseEventType::Move.to_string(), "Move");
        assert_eq!(MouseEventType::ScrollUp.to_string(), "ScrollUp");
        assert_eq!(MouseEventType::ScrollDown.to_string(), "ScrollDown");
        assert_eq!(MouseEventType::ScrollLeft.to_string(), "ScrollLeft");
        assert_eq!(MouseEventType::ScrollRight.to_string(), "ScrollRight");
    }

    // ===================================================================
    // Additional: Modifiers
    // ===================================================================

    #[test]
    fn modifiers_bits_roundtrip() {
        let m = Modifiers::SHIFT | Modifiers::CTRL;
        assert_eq!(m.bits(), 0b0101);
        let m2 = Modifiers::from_bits(m.bits());
        assert_eq!(m, m2);
    }

    #[test]
    fn modifiers_default_is_empty() {
        let m = Modifiers::default();
        assert!(m.is_empty());
    }

    // ===================================================================
    // Additional: Consumed bytes edge cases
    // ===================================================================

    #[test]
    fn consumed_bytes_minimal() {
        // Minimum valid sequence: ESC [ < 0 ; 1 ; 1 M = 9 bytes.
        let (_, len) = parse(b"\x1b[<0;1;1M").unwrap();
        assert_eq!(len, 9);
    }

    #[test]
    fn consumed_bytes_large_cb() {
        // Cb = 128 → 3 digits.
        let (_, len) = parse(b"\x1b[<128;1;1M").unwrap();
        assert_eq!(len, 11);
    }

    #[test]
    fn consumed_bytes_5digit_coords() {
        let (_, len) = parse(b"\x1b[<0;10000;20000M").unwrap();
        assert_eq!(len, 17);
    }

    // ===================================================================
    // Additional: Motion with extra buttons
    // ===================================================================

    #[test]
    fn motion_with_back_button() {
        // Cb = 128 (extra back) + 32 (motion) = 160
        let (ev, _) = parse(b"\x1b[<160;1;1M").unwrap();
        assert_eq!(ev.event_type, MouseEventType::Move);
        assert_eq!(ev.button, Button::Back);
    }

    #[test]
    fn motion_with_forward_button() {
        // Cb = 129 (extra fwd) + 32 (motion) = 161
        let (ev, _) = parse(b"\x1b[<161;1;1M").unwrap();
        assert_eq!(ev.event_type, MouseEventType::Move);
        assert_eq!(ev.button, Button::Forward);
    }
}
