//! Keyboard input parsing for the Kitty keyboard protocol.
//!
//! Parses escape sequences into structured [`KeyEvent`] values.
//! Supports press/repeat/release, modifiers, unicode, special keys,
//! and basic IME/compose handling.
//!
//! # Kitty keyboard protocol overview
//!
//! The protocol encodes key events as CSI sequences:
//! ```text
//! CSI unicode-key-code ; modifiers:event-type u
//! CSI 1 ; modifiers:event-type {A-H~}     (legacy special keys)
//! ```
//! Modifiers are encoded as `(value + 1)`, where value is a bitmask:
//! `shift=1, alt=2, ctrl=4, super=8`.
//!
//! Event types: `1` = press (default), `2` = repeat, `3` = release.

use std::fmt;

// -----------------------------------------------------------------------
// Key enum
// -----------------------------------------------------------------------

/// Identifies which key was pressed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Key {
    /// A unicode character (printable).
    Char(char),
    /// Function key F1–F24.
    F(u8),
    // Navigation
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    Delete,
    // Whitespace / control
    Enter,
    Tab,
    Backspace,
    Escape,
    // Kitty-specific
    CapsLock,
    ScrollLock,
    NumLock,
    PrintScreen,
    Pause,
    Menu,
    /// Key code that we received but don't map explicitly.
    Unknown(u32),
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Char(c) => write!(f, "{c}"),
            Self::F(n) => write!(f, "F{n}"),
            Self::Up => write!(f, "Up"),
            Self::Down => write!(f, "Down"),
            Self::Left => write!(f, "Left"),
            Self::Right => write!(f, "Right"),
            Self::Home => write!(f, "Home"),
            Self::End => write!(f, "End"),
            Self::PageUp => write!(f, "PageUp"),
            Self::PageDown => write!(f, "PageDown"),
            Self::Insert => write!(f, "Insert"),
            Self::Delete => write!(f, "Delete"),
            Self::Enter => write!(f, "Enter"),
            Self::Tab => write!(f, "Tab"),
            Self::Backspace => write!(f, "Backspace"),
            Self::Escape => write!(f, "Escape"),
            Self::CapsLock => write!(f, "CapsLock"),
            Self::ScrollLock => write!(f, "ScrollLock"),
            Self::NumLock => write!(f, "NumLock"),
            Self::PrintScreen => write!(f, "PrintScreen"),
            Self::Pause => write!(f, "Pause"),
            Self::Menu => write!(f, "Menu"),
            Self::Unknown(code) => write!(f, "Unknown({code})"),
        }
    }
}

// -----------------------------------------------------------------------
// Modifiers
// -----------------------------------------------------------------------

/// Simple bitflags macro (avoids pulling in the `bitflags` crate).
macro_rules! bitflags {
    (
        $(#[$outer:meta])*
        $vis:vis struct $name:ident : $ty:ty {
            $(
                $(#[$inner:meta])*
                const $flag:ident = $value:expr;
            )*
        }
    ) => {
        $(#[$outer])*
        $vis struct $name { bits: $ty }

        #[allow(dead_code)]
        impl $name {
            $(
                $(#[$inner])*
                pub const $flag: Self = Self { bits: $value };
            )*

            #[must_use]
            pub const fn empty() -> Self { Self { bits: 0 } }

            #[must_use]
            pub const fn from_bits_truncate(bits: $ty) -> Self {
                Self { bits }
            }

            #[must_use]
            pub const fn bits(self) -> $ty { self.bits }

            #[must_use]
            pub const fn contains(self, other: Self) -> bool {
                (self.bits & other.bits) == other.bits
            }

            #[must_use]
            pub const fn is_empty(self) -> bool {
                self.bits == 0
            }
        }

        impl ::core::ops::BitOr for $name {
            type Output = Self;
            fn bitor(self, rhs: Self) -> Self { Self { bits: self.bits | rhs.bits } }
        }

        impl ::core::ops::BitAnd for $name {
            type Output = Self;
            fn bitand(self, rhs: Self) -> Self { Self { bits: self.bits & rhs.bits } }
        }
    };
}

bitflags! {
    /// Keyboard modifier flags.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct Modifiers: u8 {
        const SHIFT = 0b0000_0001;
        const ALT   = 0b0000_0010;
        const CTRL  = 0b0000_0100;
        const SUPER = 0b0000_1000;
    }
}

// -----------------------------------------------------------------------
// Event type
// -----------------------------------------------------------------------

/// The kind of key event.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EventType {
    /// Key was pressed (or auto-detected default).
    #[default]
    Press,
    /// Key is being held (repeat).
    Repeat,
    /// Key was released.
    Release,
}

// -----------------------------------------------------------------------
// KeyEvent
// -----------------------------------------------------------------------

/// A fully parsed keyboard event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyEvent {
    /// Which key.
    pub key: Key,
    /// Active modifiers.
    pub modifiers: Modifiers,
    /// Press, repeat, or release.
    pub event_type: EventType,
}

impl KeyEvent {
    /// Create a new `KeyEvent`.
    #[must_use]
    pub fn new(key: Key, modifiers: Modifiers, event_type: EventType) -> Self {
        Self {
            key,
            modifiers,
            event_type,
        }
    }

    /// Convenience: plain key press with no modifiers.
    #[must_use]
    pub fn press(key: Key) -> Self {
        Self::new(key, Modifiers::empty(), EventType::Press)
    }
}

impl fmt::Display for KeyEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();
        if self.modifiers.contains(Modifiers::CTRL) {
            parts.push("Ctrl");
        }
        if self.modifiers.contains(Modifiers::ALT) {
            parts.push("Alt");
        }
        if self.modifiers.contains(Modifiers::SHIFT) {
            parts.push("Shift");
        }
        if self.modifiers.contains(Modifiers::SUPER) {
            parts.push("Super");
        }
        let key_str = self.key.to_string();
        parts.push(&key_str);
        write!(f, "{}", parts.join("+"))
    }
}

// -----------------------------------------------------------------------
// Parser
// -----------------------------------------------------------------------

/// Parse error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseError {
    /// Input is empty or too short.
    Empty,
    /// Not a recognised escape sequence.
    NotEscapeSequence,
    /// Malformed CSI parameters.
    MalformedParams,
    /// Unknown final byte in CSI sequence.
    UnknownFinal(u8),
    /// Incomplete sequence (need more bytes).
    Incomplete,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "empty input"),
            Self::NotEscapeSequence => write!(f, "not an escape sequence"),
            Self::MalformedParams => write!(f, "malformed CSI parameters"),
            Self::UnknownFinal(b) => write!(f, "unknown CSI final byte: {b:#04x}"),
            Self::Incomplete => write!(f, "incomplete sequence"),
        }
    }
}

/// Parse a single key event from a byte slice.
///
/// Returns the parsed event and the number of bytes consumed.
///
/// # Errors
///
/// Returns a [`ParseError`] if the input cannot be parsed.
pub fn parse(input: &[u8]) -> Result<(KeyEvent, usize), ParseError> {
    if input.is_empty() {
        return Err(ParseError::Empty);
    }

    // --- Plain single-byte characters ---
    if input[0] != 0x1b {
        return parse_plain_byte(input);
    }

    // Bare escape (no following bytes or timeout).
    if input.len() == 1 {
        return Ok((KeyEvent::press(Key::Escape), 1));
    }

    // --- CSI sequences: ESC [ ... ---
    if input.len() >= 2 && input[1] == b'[' {
        return parse_csi(input);
    }

    // --- Alt+<char>: ESC <char> ---
    if input.len() >= 2 && input[1] != b'[' && input[1] != b'O' {
        let ch = input[1] as char;
        let key = match input[1] {
            0x7f => Key::Backspace,
            0x0d => Key::Enter,
            0x09 => Key::Tab,
            _ => Key::Char(ch),
        };
        return Ok((KeyEvent::new(key, Modifiers::ALT, EventType::Press), 2));
    }

    // --- SS3 sequences: ESC O {P-S} (F1-F4) ---
    if input.len() >= 3 && input[1] == b'O' {
        return parse_ss3(input);
    }

    // ESC O with nothing after
    if input.len() == 2 && input[1] == b'O' {
        return Err(ParseError::Incomplete);
    }

    Err(ParseError::NotEscapeSequence)
}

/// Parse a non-escape byte.
fn parse_plain_byte(input: &[u8]) -> Result<(KeyEvent, usize), ParseError> {
    let b = input[0];
    match b {
        0x0d => Ok((KeyEvent::press(Key::Enter), 1)),
        0x09 => Ok((KeyEvent::press(Key::Tab), 1)),
        0x7f => Ok((KeyEvent::press(Key::Backspace), 1)),
        // Ctrl+A through Ctrl+Z (0x01–0x1a)
        0x01..=0x1a => {
            let ch = (b + b'a' - 1) as char;
            Ok((
                KeyEvent::new(Key::Char(ch), Modifiers::CTRL, EventType::Press),
                1,
            ))
        }
        // Null (Ctrl+Space or Ctrl+@)
        0x00 => Ok((
            KeyEvent::new(Key::Char(' '), Modifiers::CTRL, EventType::Press),
            1,
        )),
        // Printable ASCII / UTF-8
        _ => {
            // Try to decode a UTF-8 character.
            let s = std::str::from_utf8(input);
            match s {
                Ok(text) => {
                    let ch = text.chars().next().ok_or(ParseError::Empty)?;
                    let len = ch.len_utf8();
                    Ok((KeyEvent::press(Key::Char(ch)), len))
                }
                Err(e) => {
                    // Partial UTF-8 — try the valid prefix.
                    let valid_up_to = e.valid_up_to();
                    if valid_up_to > 0 {
                        let ch = std::str::from_utf8(&input[..valid_up_to])
                            .ok()
                            .and_then(|s| s.chars().next())
                            .ok_or(ParseError::Empty)?;
                        let len = ch.len_utf8();
                        Ok((KeyEvent::press(Key::Char(ch)), len))
                    } else if e.error_len().is_none() {
                        // Need more bytes.
                        Err(ParseError::Incomplete)
                    } else {
                        // Invalid byte — treat as unknown.
                        Ok((KeyEvent::press(Key::Unknown(u32::from(b))), 1))
                    }
                }
            }
        }
    }
}

/// Parse a CSI (`ESC [`) sequence.
fn parse_csi(input: &[u8]) -> Result<(KeyEvent, usize), ParseError> {
    // Find the final byte (0x40–0x7e).
    let csi_start = 2; // skip ESC [
    let mut end = csi_start;
    while end < input.len() {
        if input[end] >= 0x40 && input[end] <= 0x7e {
            break;
        }
        end += 1;
    }
    if end >= input.len() {
        return Err(ParseError::Incomplete);
    }

    let final_byte = input[end];
    let params_str = &input[csi_start..end];
    let consumed = end + 1;

    match final_byte {
        b'u' => parse_kitty_u(params_str, consumed),
        b'~' => parse_tilde(params_str, consumed),
        b'A' => parse_arrow_or_special(Key::Up, params_str, consumed),
        b'B' => parse_arrow_or_special(Key::Down, params_str, consumed),
        b'C' => parse_arrow_or_special(Key::Right, params_str, consumed),
        b'D' => parse_arrow_or_special(Key::Left, params_str, consumed),
        b'H' => parse_arrow_or_special(Key::Home, params_str, consumed),
        b'F' => parse_arrow_or_special(Key::End, params_str, consumed),
        b'P' => parse_arrow_or_special(Key::F(1), params_str, consumed),
        b'Q' => parse_arrow_or_special(Key::F(2), params_str, consumed),
        b'R' => parse_arrow_or_special(Key::F(3), params_str, consumed),
        b'S' => parse_arrow_or_special(Key::F(4), params_str, consumed),
        _ => Err(ParseError::UnknownFinal(final_byte)),
    }
}

/// Parse a Kitty protocol `u` sequence: `CSI code ; mods u`
fn parse_kitty_u(params: &[u8], consumed: usize) -> Result<(KeyEvent, usize), ParseError> {
    let params_s = std::str::from_utf8(params).map_err(|_| ParseError::MalformedParams)?;

    // Split by `;` into groups.
    let groups: Vec<&str> = params_s.split(';').collect();

    // First group: unicode-key-code[:shifted-key[:base-key]]
    let key_parts: Vec<&str> = groups
        .first()
        .ok_or(ParseError::MalformedParams)?
        .split(':')
        .collect();
    let key_code: u32 = key_parts
        .first()
        .ok_or(ParseError::MalformedParams)?
        .parse()
        .map_err(|_| ParseError::MalformedParams)?;

    // Second group (optional): modifiers[:event-type]
    let (modifiers, event_type) = if let Some(mod_group) = groups.get(1) {
        parse_modifiers_and_event(mod_group)?
    } else {
        (Modifiers::empty(), EventType::Press)
    };

    let key = keycode_to_key(key_code);

    Ok((KeyEvent::new(key, modifiers, event_type), consumed))
}

/// Parse modifier+event group: `modifiers[:event-type]`
fn parse_modifiers_and_event(group: &str) -> Result<(Modifiers, EventType), ParseError> {
    let parts: Vec<&str> = group.split(':').collect();

    let mod_val: u8 = parts
        .first()
        .unwrap_or(&"1")
        .parse()
        .map_err(|_| ParseError::MalformedParams)?;
    // Kitty encodes as (value + 1), so subtract 1.
    let mod_bits = mod_val.saturating_sub(1);
    let modifiers = Modifiers::from_bits_truncate(mod_bits);

    let event_type = if let Some(et) = parts.get(1) {
        match *et {
            "2" => EventType::Repeat,
            "3" => EventType::Release,
            _ => EventType::Press,
        }
    } else {
        EventType::Press
    };

    Ok((modifiers, event_type))
}

/// Parse a `~` terminated CSI sequence (legacy special keys).
fn parse_tilde(params: &[u8], consumed: usize) -> Result<(KeyEvent, usize), ParseError> {
    let params_s = std::str::from_utf8(params).map_err(|_| ParseError::MalformedParams)?;
    let groups: Vec<&str> = params_s.split(';').collect();

    let key_num: u32 = groups
        .first()
        .ok_or(ParseError::MalformedParams)?
        .parse()
        .map_err(|_| ParseError::MalformedParams)?;

    let (modifiers, event_type) = if let Some(mod_group) = groups.get(1) {
        parse_modifiers_and_event(mod_group)?
    } else {
        (Modifiers::empty(), EventType::Press)
    };

    let key = match key_num {
        1 | 7 => Key::Home,
        2 => Key::Insert,
        3 => Key::Delete,
        4 | 8 => Key::End,
        5 => Key::PageUp,
        6 => Key::PageDown,
        11 => Key::F(1),
        12 => Key::F(2),
        13 => Key::F(3),
        14 => Key::F(4),
        15 => Key::F(5),
        17 => Key::F(6),
        18 => Key::F(7),
        19 => Key::F(8),
        20 => Key::F(9),
        21 => Key::F(10),
        23 => Key::F(11),
        24 => Key::F(12),
        _ => Key::Unknown(key_num),
    };

    Ok((KeyEvent::new(key, modifiers, event_type), consumed))
}

/// Parse arrow / special key with optional modifiers: `CSI 1;mods X`
fn parse_arrow_or_special(
    key: Key,
    params: &[u8],
    consumed: usize,
) -> Result<(KeyEvent, usize), ParseError> {
    let params_s = std::str::from_utf8(params).map_err(|_| ParseError::MalformedParams)?;

    if params_s.is_empty() {
        return Ok((KeyEvent::press(key), consumed));
    }

    let groups: Vec<&str> = params_s.split(';').collect();
    let (modifiers, event_type) = if let Some(mod_group) = groups.get(1) {
        parse_modifiers_and_event(mod_group)?
    } else {
        (Modifiers::empty(), EventType::Press)
    };

    Ok((KeyEvent::new(key, modifiers, event_type), consumed))
}

/// Parse SS3 sequences: `ESC O {P,Q,R,S}` → F1–F4.
fn parse_ss3(input: &[u8]) -> Result<(KeyEvent, usize), ParseError> {
    let key = match input[2] {
        b'P' => Key::F(1),
        b'Q' => Key::F(2),
        b'R' => Key::F(3),
        b'S' => Key::F(4),
        _ => return Err(ParseError::NotEscapeSequence),
    };
    Ok((KeyEvent::press(key), 3))
}

/// Map a Kitty protocol unicode key code to our [`Key`] enum.
fn keycode_to_key(code: u32) -> Key {
    match code {
        // ASCII control codes and special keys
        8 | 127 => Key::Backspace,
        9 => Key::Tab,
        13 => Key::Enter,
        27 => Key::Escape,
        // Kitty-specific functional key codes
        57358 => Key::CapsLock,
        57359 => Key::ScrollLock,
        57360 => Key::NumLock,
        57361 => Key::PrintScreen,
        57362 => Key::Pause,
        57363 => Key::Menu,
        // Navigation (Kitty encoding for when sent via `u` terminator)
        57352 => Key::Insert,
        57353 => Key::Delete,
        57354 => Key::Home,
        57355 => Key::End,
        57356 => Key::PageUp,
        57357 => Key::PageDown,
        57350 => Key::Up,
        57351 => Key::Down,
        57349 => Key::Right,
        57348 => Key::Left,
        // Function keys (when encoded as codepoints)
        57364..=57387 => {
            // F13–F35: safe because the range guarantees (code - 57364 + 13) fits in u8
            #[allow(clippy::cast_possible_truncation)]
            let n = (code - 57364 + 13) as u8;
            Key::F(n)
        }
        // Standard unicode codepoint
        _ => char::from_u32(code).map_or(Key::Unknown(code), Key::Char),
    }
}

// -----------------------------------------------------------------------
// IME / compose helpers
// -----------------------------------------------------------------------

/// Represents a compose/IME sequence in progress.
#[derive(Clone, Debug)]
pub struct ComposeState {
    /// Characters accumulated so far.
    pub buffer: String,
    /// Whether composition is active.
    pub active: bool,
}

impl ComposeState {
    /// Create a new inactive compose state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            active: false,
        }
    }

    /// Start a compose sequence.
    pub fn start(&mut self) {
        self.active = true;
        self.buffer.clear();
    }

    /// Feed a character into the compose buffer.
    pub fn feed(&mut self, ch: char) {
        if self.active {
            self.buffer.push(ch);
        }
    }

    /// Finish composition and return the composed string.
    pub fn finish(&mut self) -> String {
        self.active = false;
        std::mem::take(&mut self.buffer)
    }

    /// Cancel composition and discard the buffer.
    pub fn cancel(&mut self) {
        self.active = false;
        self.buffer.clear();
    }
}

impl Default for ComposeState {
    fn default() -> Self {
        Self::new()
    }
}

// -----------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Plain byte parsing ---

    #[test]
    fn parse_printable_ascii() {
        let (ev, len) = parse(b"a").unwrap();
        assert_eq!(ev.key, Key::Char('a'));
        assert!(ev.modifiers.is_empty());
        assert_eq!(ev.event_type, EventType::Press);
        assert_eq!(len, 1);
    }

    #[test]
    fn parse_space() {
        let (ev, len) = parse(b" ").unwrap();
        assert_eq!(ev.key, Key::Char(' '));
        assert_eq!(len, 1);
    }

    #[test]
    fn parse_enter() {
        let (ev, len) = parse(b"\r").unwrap();
        assert_eq!(ev.key, Key::Enter);
        assert_eq!(len, 1);
    }

    #[test]
    fn parse_tab() {
        let (ev, len) = parse(b"\t").unwrap();
        assert_eq!(ev.key, Key::Tab);
        assert_eq!(len, 1);
    }

    #[test]
    fn parse_backspace() {
        let (ev, len) = parse(b"\x7f").unwrap();
        assert_eq!(ev.key, Key::Backspace);
        assert_eq!(len, 1);
    }

    #[test]
    fn parse_ctrl_a() {
        let (ev, len) = parse(b"\x01").unwrap();
        assert_eq!(ev.key, Key::Char('a'));
        assert!(ev.modifiers.contains(Modifiers::CTRL));
        assert_eq!(len, 1);
    }

    #[test]
    fn parse_ctrl_c() {
        let (ev, len) = parse(b"\x03").unwrap();
        assert_eq!(ev.key, Key::Char('c'));
        assert!(ev.modifiers.contains(Modifiers::CTRL));
        assert_eq!(len, 1);
    }

    #[test]
    fn parse_ctrl_z() {
        let (ev, len) = parse(b"\x1a").unwrap();
        assert_eq!(ev.key, Key::Char('z'));
        assert!(ev.modifiers.contains(Modifiers::CTRL));
        assert_eq!(len, 1);
    }

    #[test]
    fn parse_ctrl_space() {
        let (ev, len) = parse(b"\x00").unwrap();
        assert_eq!(ev.key, Key::Char(' '));
        assert!(ev.modifiers.contains(Modifiers::CTRL));
        assert_eq!(len, 1);
    }

    // --- UTF-8 ---

    #[test]
    fn parse_utf8_2byte() {
        // é = 0xC3 0xA9
        let (ev, len) = parse("é".as_bytes()).unwrap();
        assert_eq!(ev.key, Key::Char('é'));
        assert_eq!(len, 2);
    }

    #[test]
    fn parse_utf8_3byte() {
        // あ = 0xE3 0x81 0x82
        let (ev, len) = parse("あ".as_bytes()).unwrap();
        assert_eq!(ev.key, Key::Char('あ'));
        assert_eq!(len, 3);
    }

    #[test]
    fn parse_utf8_4byte_emoji() {
        // 🎉 = 0xF0 0x9F 0x8E 0x89
        let (ev, len) = parse("🎉".as_bytes()).unwrap();
        assert_eq!(ev.key, Key::Char('🎉'));
        assert_eq!(len, 4);
    }

    // --- Bare escape ---

    #[test]
    fn parse_bare_escape() {
        let (ev, len) = parse(b"\x1b").unwrap();
        assert_eq!(ev.key, Key::Escape);
        assert_eq!(len, 1);
    }

    // --- Alt+char ---

    #[test]
    fn parse_alt_a() {
        let (ev, len) = parse(b"\x1ba").unwrap();
        assert_eq!(ev.key, Key::Char('a'));
        assert!(ev.modifiers.contains(Modifiers::ALT));
        assert_eq!(len, 2);
    }

    #[test]
    fn parse_alt_backspace() {
        let (ev, len) = parse(b"\x1b\x7f").unwrap();
        assert_eq!(ev.key, Key::Backspace);
        assert!(ev.modifiers.contains(Modifiers::ALT));
        assert_eq!(len, 2);
    }

    // --- Arrow keys ---

    #[test]
    fn parse_arrow_up() {
        let (ev, len) = parse(b"\x1b[A").unwrap();
        assert_eq!(ev.key, Key::Up);
        assert_eq!(len, 3);
    }

    #[test]
    fn parse_arrow_down() {
        let (ev, len) = parse(b"\x1b[B").unwrap();
        assert_eq!(ev.key, Key::Down);
        assert_eq!(len, 3);
    }

    #[test]
    fn parse_arrow_right() {
        let (ev, len) = parse(b"\x1b[C").unwrap();
        assert_eq!(ev.key, Key::Right);
        assert_eq!(len, 3);
    }

    #[test]
    fn parse_arrow_left() {
        let (ev, len) = parse(b"\x1b[D").unwrap();
        assert_eq!(ev.key, Key::Left);
        assert_eq!(len, 3);
    }

    #[test]
    fn parse_home() {
        let (ev, _) = parse(b"\x1b[H").unwrap();
        assert_eq!(ev.key, Key::Home);
    }

    #[test]
    fn parse_end() {
        let (ev, _) = parse(b"\x1b[F").unwrap();
        assert_eq!(ev.key, Key::End);
    }

    // --- Arrow keys with modifiers ---

    #[test]
    fn parse_shift_up() {
        let (ev, _) = parse(b"\x1b[1;2A").unwrap();
        assert_eq!(ev.key, Key::Up);
        assert!(ev.modifiers.contains(Modifiers::SHIFT));
    }

    #[test]
    fn parse_ctrl_left() {
        let (ev, _) = parse(b"\x1b[1;5D").unwrap();
        assert_eq!(ev.key, Key::Left);
        assert!(ev.modifiers.contains(Modifiers::CTRL));
    }

    #[test]
    fn parse_alt_right() {
        let (ev, _) = parse(b"\x1b[1;3C").unwrap();
        assert_eq!(ev.key, Key::Right);
        assert!(ev.modifiers.contains(Modifiers::ALT));
    }

    #[test]
    fn parse_ctrl_shift_up() {
        let (ev, _) = parse(b"\x1b[1;6A").unwrap();
        assert_eq!(ev.key, Key::Up);
        assert!(ev.modifiers.contains(Modifiers::CTRL));
        assert!(ev.modifiers.contains(Modifiers::SHIFT));
    }

    // --- Tilde sequences ---

    #[test]
    fn parse_insert() {
        let (ev, _) = parse(b"\x1b[2~").unwrap();
        assert_eq!(ev.key, Key::Insert);
    }

    #[test]
    fn parse_delete() {
        let (ev, _) = parse(b"\x1b[3~").unwrap();
        assert_eq!(ev.key, Key::Delete);
    }

    #[test]
    fn parse_page_up() {
        let (ev, _) = parse(b"\x1b[5~").unwrap();
        assert_eq!(ev.key, Key::PageUp);
    }

    #[test]
    fn parse_page_down() {
        let (ev, _) = parse(b"\x1b[6~").unwrap();
        assert_eq!(ev.key, Key::PageDown);
    }

    #[test]
    fn parse_f5() {
        let (ev, _) = parse(b"\x1b[15~").unwrap();
        assert_eq!(ev.key, Key::F(5));
    }

    #[test]
    fn parse_f12() {
        let (ev, _) = parse(b"\x1b[24~").unwrap();
        assert_eq!(ev.key, Key::F(12));
    }

    #[test]
    fn parse_delete_with_shift() {
        let (ev, _) = parse(b"\x1b[3;2~").unwrap();
        assert_eq!(ev.key, Key::Delete);
        assert!(ev.modifiers.contains(Modifiers::SHIFT));
    }

    // --- SS3 sequences (F1-F4) ---

    #[test]
    fn parse_ss3_f1() {
        let (ev, len) = parse(b"\x1bOP").unwrap();
        assert_eq!(ev.key, Key::F(1));
        assert_eq!(len, 3);
    }

    #[test]
    fn parse_ss3_f4() {
        let (ev, len) = parse(b"\x1bOS").unwrap();
        assert_eq!(ev.key, Key::F(4));
        assert_eq!(len, 3);
    }

    // --- Kitty protocol `u` sequences ---

    #[test]
    fn parse_kitty_plain_a() {
        // CSI 97 u → 'a'
        let (ev, _) = parse(b"\x1b[97u").unwrap();
        assert_eq!(ev.key, Key::Char('a'));
        assert!(ev.modifiers.is_empty());
        assert_eq!(ev.event_type, EventType::Press);
    }

    #[test]
    fn parse_kitty_shift_a() {
        // CSI 97;2u → Shift+'a'
        let (ev, _) = parse(b"\x1b[97;2u").unwrap();
        assert_eq!(ev.key, Key::Char('a'));
        assert!(ev.modifiers.contains(Modifiers::SHIFT));
    }

    #[test]
    fn parse_kitty_ctrl_a() {
        // CSI 97;5u → Ctrl+'a'
        let (ev, _) = parse(b"\x1b[97;5u").unwrap();
        assert_eq!(ev.key, Key::Char('a'));
        assert!(ev.modifiers.contains(Modifiers::CTRL));
    }

    #[test]
    fn parse_kitty_ctrl_shift_a() {
        // CSI 97;6u → Ctrl+Shift+'a'
        let (ev, _) = parse(b"\x1b[97;6u").unwrap();
        assert_eq!(ev.key, Key::Char('a'));
        assert!(ev.modifiers.contains(Modifiers::CTRL));
        assert!(ev.modifiers.contains(Modifiers::SHIFT));
    }

    #[test]
    fn parse_kitty_super_a() {
        // CSI 97;9u → Super+'a'
        let (ev, _) = parse(b"\x1b[97;9u").unwrap();
        assert_eq!(ev.key, Key::Char('a'));
        assert!(ev.modifiers.contains(Modifiers::SUPER));
    }

    #[test]
    fn parse_kitty_enter() {
        // CSI 13u → Enter
        let (ev, _) = parse(b"\x1b[13u").unwrap();
        assert_eq!(ev.key, Key::Enter);
    }

    #[test]
    fn parse_kitty_escape() {
        let (ev, _) = parse(b"\x1b[27u").unwrap();
        assert_eq!(ev.key, Key::Escape);
    }

    #[test]
    fn parse_kitty_tab() {
        let (ev, _) = parse(b"\x1b[9u").unwrap();
        assert_eq!(ev.key, Key::Tab);
    }

    #[test]
    fn parse_kitty_backspace() {
        let (ev, _) = parse(b"\x1b[127u").unwrap();
        assert_eq!(ev.key, Key::Backspace);
    }

    // --- Event types ---

    #[test]
    fn parse_kitty_key_repeat() {
        // CSI 97;1:2u → 'a' repeat
        let (ev, _) = parse(b"\x1b[97;1:2u").unwrap();
        assert_eq!(ev.key, Key::Char('a'));
        assert_eq!(ev.event_type, EventType::Repeat);
    }

    #[test]
    fn parse_kitty_key_release() {
        // CSI 97;1:3u → 'a' release
        let (ev, _) = parse(b"\x1b[97;1:3u").unwrap();
        assert_eq!(ev.key, Key::Char('a'));
        assert_eq!(ev.event_type, EventType::Release);
    }

    #[test]
    fn parse_kitty_ctrl_release() {
        // CSI 97;5:3u → Ctrl+'a' release
        let (ev, _) = parse(b"\x1b[97;5:3u").unwrap();
        assert_eq!(ev.key, Key::Char('a'));
        assert!(ev.modifiers.contains(Modifiers::CTRL));
        assert_eq!(ev.event_type, EventType::Release);
    }

    // --- Unicode via kitty protocol ---

    #[test]
    fn parse_kitty_unicode_e_acute() {
        // é = U+00E9 = 233
        let (ev, _) = parse(b"\x1b[233u").unwrap();
        assert_eq!(ev.key, Key::Char('\u{00E9}'));
    }

    #[test]
    fn parse_kitty_unicode_cjk() {
        // 中 = U+4E2D = 20013
        let (ev, _) = parse(b"\x1b[20013u").unwrap();
        assert_eq!(ev.key, Key::Char('中'));
    }

    // --- Error cases ---

    #[test]
    fn parse_empty_returns_error() {
        assert_eq!(parse(b""), Err(ParseError::Empty));
    }

    #[test]
    fn parse_incomplete_csi() {
        assert_eq!(parse(b"\x1b["), Err(ParseError::Incomplete));
    }

    #[test]
    fn parse_incomplete_csi_params() {
        assert_eq!(parse(b"\x1b[1;"), Err(ParseError::Incomplete));
    }

    // --- Modifiers struct ---

    #[test]
    fn modifiers_empty() {
        let m = Modifiers::empty();
        assert!(m.is_empty());
        assert!(!m.contains(Modifiers::SHIFT));
    }

    #[test]
    fn modifiers_combine() {
        let m = Modifiers::CTRL | Modifiers::ALT;
        assert!(m.contains(Modifiers::CTRL));
        assert!(m.contains(Modifiers::ALT));
        assert!(!m.contains(Modifiers::SHIFT));
    }

    #[test]
    fn modifiers_from_bits() {
        // ctrl=4, shift=1 → 5
        let m = Modifiers::from_bits_truncate(5);
        assert!(m.contains(Modifiers::CTRL));
        assert!(m.contains(Modifiers::SHIFT));
        assert!(!m.contains(Modifiers::ALT));
    }

    // --- KeyEvent display ---

    #[test]
    fn key_event_display_plain() {
        let ev = KeyEvent::press(Key::Char('a'));
        assert_eq!(ev.to_string(), "a");
    }

    #[test]
    fn key_event_display_ctrl_c() {
        let ev = KeyEvent::new(Key::Char('c'), Modifiers::CTRL, EventType::Press);
        assert_eq!(ev.to_string(), "Ctrl+c");
    }

    #[test]
    fn key_event_display_ctrl_shift_up() {
        let ev = KeyEvent::new(
            Key::Up,
            Modifiers::CTRL | Modifiers::SHIFT,
            EventType::Press,
        );
        assert_eq!(ev.to_string(), "Ctrl+Shift+Up");
    }

    // --- Compose state ---

    #[test]
    fn compose_lifecycle() {
        let mut cs = ComposeState::new();
        assert!(!cs.active);

        cs.start();
        assert!(cs.active);

        cs.feed('e');
        cs.feed('\u{0301}'); // combining acute accent
        assert_eq!(cs.buffer, "e\u{0301}");

        let result = cs.finish();
        assert_eq!(result, "e\u{0301}");
        assert!(!cs.active);
        assert!(cs.buffer.is_empty());
    }

    #[test]
    fn compose_cancel() {
        let mut cs = ComposeState::new();
        cs.start();
        cs.feed('a');
        cs.cancel();
        assert!(!cs.active);
        assert!(cs.buffer.is_empty());
    }

    #[test]
    fn compose_feed_ignored_when_inactive() {
        let mut cs = ComposeState::new();
        cs.feed('x');
        assert!(cs.buffer.is_empty());
    }

    // --- Consumed bytes ---

    #[test]
    fn consumed_bytes_plain() {
        let input = b"abc";
        let (_, len) = parse(input).unwrap();
        assert_eq!(len, 1); // only consumes 'a'
    }

    #[test]
    fn consumed_bytes_csi() {
        let input = b"\x1b[Amore";
        let (ev, len) = parse(input).unwrap();
        assert_eq!(ev.key, Key::Up);
        assert_eq!(len, 3); // ESC [ A
    }

    #[test]
    fn consumed_bytes_kitty_u() {
        let input = b"\x1b[97;5:3urest";
        let (_, len) = parse(input).unwrap();
        assert_eq!(len, 9); // ESC [ 97;5:3 u
    }
}
