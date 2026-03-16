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

    // ===================================================================
    // 1. Malformed / incomplete sequences
    // ===================================================================

    #[test]
    fn malformed_csi_no_final_byte() {
        // CSI with only intermediate params, no final byte.
        assert_eq!(parse(b"\x1b[1;2"), Err(ParseError::Incomplete));
    }

    #[test]
    fn malformed_csi_non_numeric_params() {
        // 'a' (0x61) is in the final byte range (0x40-0x7e), so CSI parser
        // sees it as the final byte, not part of params. Unknown final 'a'.
        assert_eq!(parse(b"\x1b[abcu"), Err(ParseError::UnknownFinal(b'a')));
    }

    #[test]
    fn malformed_tilde_non_numeric() {
        // 'x' (0x78) is a final byte, so parser dispatches on 'x'.
        assert_eq!(parse(b"\x1b[xy~"), Err(ParseError::UnknownFinal(b'x')));
    }

    #[test]
    fn malformed_kitty_u_bad_keycode() {
        // Valid CSI with 'u' final but non-numeric first param.
        // The intermediate bytes are all < 0x40, so parser reaches 'u' as final.
        // But "1a" can't parse as u32.
        assert_eq!(parse(b"\x1b[1a;2u"), Err(ParseError::UnknownFinal(b'a')));
    }

    #[test]
    fn malformed_tilde_empty_params() {
        // CSI with just ~ and no params — empty string can't parse as u32.
        assert_eq!(parse(b"\x1b[~"), Err(ParseError::MalformedParams));
    }

    #[test]
    fn truncated_escape_only() {
        // Just ESC → bare escape key.
        let (ev, len) = parse(b"\x1b").unwrap();
        assert_eq!(ev.key, Key::Escape);
        assert_eq!(len, 1);
    }

    #[test]
    fn truncated_esc_o_only() {
        // ESC O with no follow-up byte.
        assert_eq!(parse(b"\x1bO"), Err(ParseError::Incomplete));
    }

    #[test]
    fn unknown_csi_final_byte() {
        let result = parse(b"\x1b[1;2Z");
        assert_eq!(result, Err(ParseError::UnknownFinal(b'Z')));
    }

    #[test]
    fn garbage_byte_0xff() {
        let (ev, len) = parse(b"\xff").unwrap();
        assert_eq!(ev.key, Key::Unknown(0xff));
        assert_eq!(len, 1);
    }

    #[test]
    fn garbage_byte_0xfe() {
        let (ev, len) = parse(b"\xfe").unwrap();
        assert_eq!(ev.key, Key::Unknown(0xfe));
        assert_eq!(len, 1);
    }

    #[test]
    fn incomplete_utf8_leading_byte_only() {
        // 0xC3 is a 2-byte UTF-8 leader but no continuation.
        assert_eq!(parse(&[0xC3]), Err(ParseError::Incomplete));
    }

    #[test]
    fn incomplete_utf8_3byte_missing_one() {
        // 0xE3 0x81 is start of a 3-byte char, missing final byte.
        assert_eq!(parse(&[0xE3, 0x81]), Err(ParseError::Incomplete));
    }

    #[test]
    fn incomplete_utf8_4byte_missing_two() {
        assert_eq!(parse(&[0xF0, 0x9F]), Err(ParseError::Incomplete));
    }

    // ===================================================================
    // 2. Boundary conditions
    // ===================================================================

    #[test]
    fn single_byte_nul() {
        let (ev, len) = parse(b"\x00").unwrap();
        assert_eq!(ev.key, Key::Char(' '));
        assert!(ev.modifiers.contains(Modifiers::CTRL));
        assert_eq!(len, 1);
    }

    #[test]
    fn single_byte_max_ascii() {
        // DEL (0x7f) = Backspace.
        let (ev, _) = parse(b"\x7f").unwrap();
        assert_eq!(ev.key, Key::Backspace);
    }

    #[test]
    fn very_long_kitty_codepoint() {
        // Large valid unicode codepoint via Kitty protocol.
        // U+1F600 = 128512
        let (ev, _) = parse(b"\x1b[128512u").unwrap();
        assert_eq!(ev.key, Key::Char('\u{1F600}'));
    }

    #[test]
    fn invalid_unicode_codepoint() {
        // 0xD800 is a surrogate, not valid char.
        let (ev, _) = parse(b"\x1b[55296u").unwrap();
        assert_eq!(ev.key, Key::Unknown(55296));
    }

    #[test]
    fn codepoint_beyond_unicode_range() {
        // 0x110000 is beyond Unicode max.
        let (ev, _) = parse(b"\x1b[1114112u").unwrap();
        assert_eq!(ev.key, Key::Unknown(1_114_112));
    }

    // ===================================================================
    // 3. All modifier combinations
    // ===================================================================

    #[test]
    fn modifier_shift_alone() {
        // Kitty: mod 2 = shift (2-1=1=shift)
        let (ev, _) = parse(b"\x1b[97;2u").unwrap();
        assert_eq!(ev.modifiers, Modifiers::SHIFT);
    }

    #[test]
    fn modifier_alt_alone() {
        // mod 3 = alt (3-1=2=alt)
        let (ev, _) = parse(b"\x1b[97;3u").unwrap();
        assert_eq!(ev.modifiers, Modifiers::ALT);
    }

    #[test]
    fn modifier_ctrl_alone() {
        // mod 5 = ctrl (5-1=4=ctrl)
        let (ev, _) = parse(b"\x1b[97;5u").unwrap();
        assert_eq!(ev.modifiers, Modifiers::CTRL);
    }

    #[test]
    fn modifier_super_alone() {
        // mod 9 = super (9-1=8=super)
        let (ev, _) = parse(b"\x1b[97;9u").unwrap();
        assert_eq!(ev.modifiers, Modifiers::SUPER);
    }

    #[test]
    fn modifier_shift_alt() {
        // mod 4 = shift+alt (4-1=3=shift|alt)
        let (ev, _) = parse(b"\x1b[97;4u").unwrap();
        assert!(ev.modifiers.contains(Modifiers::SHIFT));
        assert!(ev.modifiers.contains(Modifiers::ALT));
        assert!(!ev.modifiers.contains(Modifiers::CTRL));
    }

    #[test]
    fn modifier_shift_ctrl() {
        // mod 6 = shift+ctrl (6-1=5=shift|ctrl)
        let (ev, _) = parse(b"\x1b[97;6u").unwrap();
        assert!(ev.modifiers.contains(Modifiers::SHIFT));
        assert!(ev.modifiers.contains(Modifiers::CTRL));
    }

    #[test]
    fn modifier_alt_ctrl() {
        // mod 7 = alt+ctrl (7-1=6=alt|ctrl)
        let (ev, _) = parse(b"\x1b[97;7u").unwrap();
        assert!(ev.modifiers.contains(Modifiers::ALT));
        assert!(ev.modifiers.contains(Modifiers::CTRL));
    }

    #[test]
    fn modifier_shift_alt_ctrl() {
        // mod 8 = shift+alt+ctrl (8-1=7)
        let (ev, _) = parse(b"\x1b[97;8u").unwrap();
        assert!(ev.modifiers.contains(Modifiers::SHIFT));
        assert!(ev.modifiers.contains(Modifiers::ALT));
        assert!(ev.modifiers.contains(Modifiers::CTRL));
        assert!(!ev.modifiers.contains(Modifiers::SUPER));
    }

    #[test]
    fn modifier_all_four() {
        // mod 16 = all (16-1=15=shift|alt|ctrl|super)
        let (ev, _) = parse(b"\x1b[97;16u").unwrap();
        assert!(ev.modifiers.contains(Modifiers::SHIFT));
        assert!(ev.modifiers.contains(Modifiers::ALT));
        assert!(ev.modifiers.contains(Modifiers::CTRL));
        assert!(ev.modifiers.contains(Modifiers::SUPER));
    }

    #[test]
    fn modifier_ctrl_super() {
        // mod 13 = ctrl+super (13-1=12=ctrl|super=4|8)
        let (ev, _) = parse(b"\x1b[97;13u").unwrap();
        assert!(ev.modifiers.contains(Modifiers::CTRL));
        assert!(ev.modifiers.contains(Modifiers::SUPER));
        assert!(!ev.modifiers.contains(Modifiers::SHIFT));
        assert!(!ev.modifiers.contains(Modifiers::ALT));
    }

    #[test]
    fn modifier_shift_with_arrow() {
        let (ev, _) = parse(b"\x1b[1;2B").unwrap();
        assert_eq!(ev.key, Key::Down);
        assert_eq!(ev.modifiers, Modifiers::SHIFT);
    }

    #[test]
    fn modifier_ctrl_with_home() {
        let (ev, _) = parse(b"\x1b[1;5H").unwrap();
        assert_eq!(ev.key, Key::Home);
        assert!(ev.modifiers.contains(Modifiers::CTRL));
    }

    #[test]
    fn modifier_alt_with_end() {
        let (ev, _) = parse(b"\x1b[1;3F").unwrap();
        assert_eq!(ev.key, Key::End);
        assert!(ev.modifiers.contains(Modifiers::ALT));
    }

    #[test]
    fn modifier_shift_with_tilde_delete() {
        let (ev, _) = parse(b"\x1b[3;2~").unwrap();
        assert_eq!(ev.key, Key::Delete);
        assert!(ev.modifiers.contains(Modifiers::SHIFT));
    }

    #[test]
    fn modifier_ctrl_with_tilde_pageup() {
        let (ev, _) = parse(b"\x1b[5;5~").unwrap();
        assert_eq!(ev.key, Key::PageUp);
        assert!(ev.modifiers.contains(Modifiers::CTRL));
    }

    // ===================================================================
    // 4. Unicode edge cases
    // ===================================================================

    #[test]
    fn utf8_2byte_latin_supplement() {
        // ñ = U+00F1
        let (ev, len) = parse("ñ".as_bytes()).unwrap();
        assert_eq!(ev.key, Key::Char('ñ'));
        assert_eq!(len, 2);
    }

    #[test]
    fn utf8_3byte_korean() {
        // 한 = U+D55C
        let (ev, len) = parse("한".as_bytes()).unwrap();
        assert_eq!(ev.key, Key::Char('한'));
        assert_eq!(len, 3);
    }

    #[test]
    fn utf8_4byte_math_symbol() {
        // 𝄞 (musical symbol G clef) = U+1D11E
        let (ev, len) = parse("𝄞".as_bytes()).unwrap();
        assert_eq!(ev.key, Key::Char('𝄞'));
        assert_eq!(len, 4);
    }

    #[test]
    fn utf8_4byte_emoji_face() {
        // 😀 = U+1F600
        let (ev, len) = parse("😀".as_bytes()).unwrap();
        assert_eq!(ev.key, Key::Char('😀'));
        assert_eq!(len, 4);
    }

    #[test]
    fn utf8_combining_accent() {
        // e followed by combining acute: "e\u{0301}" — parser only reads first char.
        let input = "e\u{0301}".as_bytes();
        let (ev, len) = parse(input).unwrap();
        assert_eq!(ev.key, Key::Char('e'));
        assert_eq!(len, 1);
    }

    #[test]
    fn utf8_bom_character() {
        // U+FEFF BOM
        let (ev, len) = parse("\u{FEFF}".as_bytes()).unwrap();
        assert_eq!(ev.key, Key::Char('\u{FEFF}'));
        assert_eq!(len, 3);
    }

    #[test]
    fn kitty_emoji_codepoint() {
        // 🎉 = U+1F389 = 127881 via Kitty protocol.
        let (ev, _) = parse(b"\x1b[127881u").unwrap();
        assert_eq!(ev.key, Key::Char('🎉'));
    }

    #[test]
    fn utf8_multibyte_followed_by_more() {
        // Parse only first char from multi-char input.
        let input = "éàü".as_bytes();
        let (ev, len) = parse(input).unwrap();
        assert_eq!(ev.key, Key::Char('é'));
        assert_eq!(len, 2);
    }

    // ===================================================================
    // 5. Kitty protocol specifics
    // ===================================================================

    #[test]
    fn kitty_shifted_key_reporting() {
        // CSI 97:65u → 'a' with shifted key 'A' (65).
        let (ev, _) = parse(b"\x1b[97:65u").unwrap();
        assert_eq!(ev.key, Key::Char('a'));
    }

    #[test]
    fn kitty_shifted_and_base_key() {
        // CSI 97:65:97u → key='a', shifted='A', base='a'.
        let (ev, _) = parse(b"\x1b[97:65:97u").unwrap();
        assert_eq!(ev.key, Key::Char('a'));
    }

    #[test]
    fn kitty_press_event_explicit() {
        // CSI 97;1:1u → explicit press event.
        let (ev, _) = parse(b"\x1b[97;1:1u").unwrap();
        assert_eq!(ev.key, Key::Char('a'));
        assert_eq!(ev.event_type, EventType::Press);
    }

    #[test]
    fn kitty_repeat_event() {
        let (ev, _) = parse(b"\x1b[97;1:2u").unwrap();
        assert_eq!(ev.event_type, EventType::Repeat);
    }

    #[test]
    fn kitty_release_event() {
        let (ev, _) = parse(b"\x1b[97;1:3u").unwrap();
        assert_eq!(ev.event_type, EventType::Release);
    }

    #[test]
    fn kitty_ctrl_repeat() {
        // CSI 97;5:2u → Ctrl+'a' repeat
        let (ev, _) = parse(b"\x1b[97;5:2u").unwrap();
        assert!(ev.modifiers.contains(Modifiers::CTRL));
        assert_eq!(ev.event_type, EventType::Repeat);
    }

    #[test]
    fn kitty_all_mods_release() {
        // CSI 97;16:3u → all modifiers + release.
        let (ev, _) = parse(b"\x1b[97;16:3u").unwrap();
        assert!(ev.modifiers.contains(Modifiers::SHIFT));
        assert!(ev.modifiers.contains(Modifiers::ALT));
        assert!(ev.modifiers.contains(Modifiers::CTRL));
        assert!(ev.modifiers.contains(Modifiers::SUPER));
        assert_eq!(ev.event_type, EventType::Release);
    }

    #[test]
    fn kitty_special_capslock() {
        let (ev, _) = parse(b"\x1b[57358u").unwrap();
        assert_eq!(ev.key, Key::CapsLock);
    }

    #[test]
    fn kitty_special_scrolllock() {
        let (ev, _) = parse(b"\x1b[57359u").unwrap();
        assert_eq!(ev.key, Key::ScrollLock);
    }

    #[test]
    fn kitty_special_numlock() {
        let (ev, _) = parse(b"\x1b[57360u").unwrap();
        assert_eq!(ev.key, Key::NumLock);
    }

    #[test]
    fn kitty_special_printscreen() {
        let (ev, _) = parse(b"\x1b[57361u").unwrap();
        assert_eq!(ev.key, Key::PrintScreen);
    }

    #[test]
    fn kitty_special_pause() {
        let (ev, _) = parse(b"\x1b[57362u").unwrap();
        assert_eq!(ev.key, Key::Pause);
    }

    #[test]
    fn kitty_special_menu() {
        let (ev, _) = parse(b"\x1b[57363u").unwrap();
        assert_eq!(ev.key, Key::Menu);
    }

    #[test]
    fn kitty_nav_insert() {
        let (ev, _) = parse(b"\x1b[57352u").unwrap();
        assert_eq!(ev.key, Key::Insert);
    }

    #[test]
    fn kitty_nav_delete() {
        let (ev, _) = parse(b"\x1b[57353u").unwrap();
        assert_eq!(ev.key, Key::Delete);
    }

    #[test]
    fn kitty_nav_home() {
        let (ev, _) = parse(b"\x1b[57354u").unwrap();
        assert_eq!(ev.key, Key::Home);
    }

    #[test]
    fn kitty_nav_end() {
        let (ev, _) = parse(b"\x1b[57355u").unwrap();
        assert_eq!(ev.key, Key::End);
    }

    #[test]
    fn kitty_nav_pageup() {
        let (ev, _) = parse(b"\x1b[57356u").unwrap();
        assert_eq!(ev.key, Key::PageUp);
    }

    #[test]
    fn kitty_nav_pagedown() {
        let (ev, _) = parse(b"\x1b[57357u").unwrap();
        assert_eq!(ev.key, Key::PageDown);
    }

    #[test]
    fn kitty_nav_up() {
        let (ev, _) = parse(b"\x1b[57350u").unwrap();
        assert_eq!(ev.key, Key::Up);
    }

    #[test]
    fn kitty_nav_down() {
        let (ev, _) = parse(b"\x1b[57351u").unwrap();
        assert_eq!(ev.key, Key::Down);
    }

    #[test]
    fn kitty_nav_right() {
        let (ev, _) = parse(b"\x1b[57349u").unwrap();
        assert_eq!(ev.key, Key::Right);
    }

    #[test]
    fn kitty_nav_left() {
        let (ev, _) = parse(b"\x1b[57348u").unwrap();
        assert_eq!(ev.key, Key::Left);
    }

    #[test]
    fn kitty_backspace_codepoint_8() {
        let (ev, _) = parse(b"\x1b[8u").unwrap();
        assert_eq!(ev.key, Key::Backspace);
    }

    // ===================================================================
    // 6. Legacy sequence disambiguation
    // ===================================================================

    #[test]
    fn legacy_home_tilde_7() {
        // Some terminals send ESC[7~ for Home.
        let (ev, _) = parse(b"\x1b[7~").unwrap();
        assert_eq!(ev.key, Key::Home);
    }

    #[test]
    fn legacy_end_tilde_8() {
        let (ev, _) = parse(b"\x1b[8~").unwrap();
        assert_eq!(ev.key, Key::End);
    }

    #[test]
    fn legacy_home_tilde_1() {
        let (ev, _) = parse(b"\x1b[1~").unwrap();
        assert_eq!(ev.key, Key::Home);
    }

    #[test]
    fn legacy_end_tilde_4() {
        let (ev, _) = parse(b"\x1b[4~").unwrap();
        assert_eq!(ev.key, Key::End);
    }

    #[test]
    fn legacy_f1_tilde_11() {
        let (ev, _) = parse(b"\x1b[11~").unwrap();
        assert_eq!(ev.key, Key::F(1));
    }

    #[test]
    fn legacy_f4_tilde_14() {
        let (ev, _) = parse(b"\x1b[14~").unwrap();
        assert_eq!(ev.key, Key::F(4));
    }

    #[test]
    fn csi_f1_via_p() {
        // CSI P = F1 (xterm encoding).
        let (ev, _) = parse(b"\x1b[P").unwrap();
        assert_eq!(ev.key, Key::F(1));
    }

    #[test]
    fn csi_f2_via_q() {
        let (ev, _) = parse(b"\x1b[Q").unwrap();
        assert_eq!(ev.key, Key::F(2));
    }

    #[test]
    fn csi_f3_via_r() {
        let (ev, _) = parse(b"\x1b[R").unwrap();
        assert_eq!(ev.key, Key::F(3));
    }

    #[test]
    fn csi_f4_via_s() {
        let (ev, _) = parse(b"\x1b[S").unwrap();
        assert_eq!(ev.key, Key::F(4));
    }

    #[test]
    fn unknown_tilde_key_number() {
        let (ev, _) = parse(b"\x1b[99~").unwrap();
        assert_eq!(ev.key, Key::Unknown(99));
    }

    // ===================================================================
    // 7. Rapid sequential events (multiple in one buffer)
    // ===================================================================

    #[test]
    fn sequential_two_plain_chars() {
        let input = b"ab";
        let (ev1, len1) = parse(input).unwrap();
        assert_eq!(ev1.key, Key::Char('a'));
        assert_eq!(len1, 1);
        let (ev2, len2) = parse(&input[len1..]).unwrap();
        assert_eq!(ev2.key, Key::Char('b'));
        assert_eq!(len2, 1);
    }

    #[test]
    fn sequential_csi_then_plain() {
        let input = b"\x1b[Ax";
        let (ev1, len1) = parse(input).unwrap();
        assert_eq!(ev1.key, Key::Up);
        assert_eq!(len1, 3);
        let (ev2, _) = parse(&input[len1..]).unwrap();
        assert_eq!(ev2.key, Key::Char('x'));
    }

    #[test]
    fn sequential_kitty_events() {
        let input = b"\x1b[97;1:1u\x1b[97;1:2u\x1b[97;1:3u";
        let (ev1, l1) = parse(input).unwrap();
        assert_eq!(ev1.event_type, EventType::Press);
        let (ev2, l2) = parse(&input[l1..]).unwrap();
        assert_eq!(ev2.event_type, EventType::Repeat);
        let (ev3, _) = parse(&input[l1 + l2..]).unwrap();
        assert_eq!(ev3.event_type, EventType::Release);
    }

    #[test]
    fn sequential_mixed_utf8_and_csi() {
        let mut input = Vec::new();
        input.extend_from_slice("é".as_bytes());
        input.extend_from_slice(b"\x1b[B");
        input.extend_from_slice("あ".as_bytes());

        let (ev1, l1) = parse(&input).unwrap();
        assert_eq!(ev1.key, Key::Char('é'));
        assert_eq!(l1, 2);

        let (ev2, l2) = parse(&input[l1..]).unwrap();
        assert_eq!(ev2.key, Key::Down);
        assert_eq!(l2, 3);

        let (ev3, l3) = parse(&input[l1 + l2..]).unwrap();
        assert_eq!(ev3.key, Key::Char('あ'));
        assert_eq!(l3, 3);
    }

    #[test]
    fn sequential_three_arrows() {
        let input = b"\x1b[A\x1b[B\x1b[C";
        let (ev1, l1) = parse(input).unwrap();
        assert_eq!(ev1.key, Key::Up);
        let (ev2, l2) = parse(&input[l1..]).unwrap();
        assert_eq!(ev2.key, Key::Down);
        let (ev3, _) = parse(&input[l1 + l2..]).unwrap();
        assert_eq!(ev3.key, Key::Right);
    }

    // ===================================================================
    // 8. All special keys in Key enum
    // ===================================================================

    // Most are tested above, but ensure explicit coverage for every variant.

    #[test]
    fn key_up_csi() {
        let (ev, _) = parse(b"\x1b[A").unwrap();
        assert_eq!(ev.key, Key::Up);
    }

    #[test]
    fn key_down_csi() {
        let (ev, _) = parse(b"\x1b[B").unwrap();
        assert_eq!(ev.key, Key::Down);
    }

    #[test]
    fn key_right_csi() {
        let (ev, _) = parse(b"\x1b[C").unwrap();
        assert_eq!(ev.key, Key::Right);
    }

    #[test]
    fn key_left_csi() {
        let (ev, _) = parse(b"\x1b[D").unwrap();
        assert_eq!(ev.key, Key::Left);
    }

    #[test]
    fn key_home_csi() {
        let (ev, _) = parse(b"\x1b[H").unwrap();
        assert_eq!(ev.key, Key::Home);
    }

    #[test]
    fn key_end_csi() {
        let (ev, _) = parse(b"\x1b[F").unwrap();
        assert_eq!(ev.key, Key::End);
    }

    #[test]
    fn key_insert_tilde() {
        let (ev, _) = parse(b"\x1b[2~").unwrap();
        assert_eq!(ev.key, Key::Insert);
    }

    #[test]
    fn key_delete_tilde() {
        let (ev, _) = parse(b"\x1b[3~").unwrap();
        assert_eq!(ev.key, Key::Delete);
    }

    #[test]
    fn key_pageup_tilde() {
        let (ev, _) = parse(b"\x1b[5~").unwrap();
        assert_eq!(ev.key, Key::PageUp);
    }

    #[test]
    fn key_pagedown_tilde() {
        let (ev, _) = parse(b"\x1b[6~").unwrap();
        assert_eq!(ev.key, Key::PageDown);
    }

    #[test]
    fn key_enter_byte() {
        let (ev, _) = parse(b"\r").unwrap();
        assert_eq!(ev.key, Key::Enter);
    }

    #[test]
    fn key_tab_byte() {
        let (ev, _) = parse(b"\t").unwrap();
        assert_eq!(ev.key, Key::Tab);
    }

    #[test]
    fn key_backspace_byte() {
        let (ev, _) = parse(b"\x7f").unwrap();
        assert_eq!(ev.key, Key::Backspace);
    }

    #[test]
    fn key_escape_bare() {
        let (ev, _) = parse(b"\x1b").unwrap();
        assert_eq!(ev.key, Key::Escape);
    }

    #[test]
    fn key_unknown_variant() {
        // Unknown tilde code.
        let (ev, _) = parse(b"\x1b[42~").unwrap();
        assert_eq!(ev.key, Key::Unknown(42));
    }

    // ===================================================================
    // 9. Function keys F1–F24
    // ===================================================================

    // F1-F4 via SS3.
    #[test]
    fn ss3_f1() {
        let (ev, _) = parse(b"\x1bOP").unwrap();
        assert_eq!(ev.key, Key::F(1));
    }

    #[test]
    fn ss3_f2() {
        let (ev, _) = parse(b"\x1bOQ").unwrap();
        assert_eq!(ev.key, Key::F(2));
    }

    #[test]
    fn ss3_f3() {
        let (ev, _) = parse(b"\x1bOR").unwrap();
        assert_eq!(ev.key, Key::F(3));
    }

    #[test]
    fn ss3_f4() {
        let (ev, _) = parse(b"\x1bOS").unwrap();
        assert_eq!(ev.key, Key::F(4));
    }

    // F1-F4 via tilde encoding.
    #[test]
    fn tilde_f1() {
        let (ev, _) = parse(b"\x1b[11~").unwrap();
        assert_eq!(ev.key, Key::F(1));
    }

    #[test]
    fn tilde_f2() {
        let (ev, _) = parse(b"\x1b[12~").unwrap();
        assert_eq!(ev.key, Key::F(2));
    }

    #[test]
    fn tilde_f3() {
        let (ev, _) = parse(b"\x1b[13~").unwrap();
        assert_eq!(ev.key, Key::F(3));
    }

    #[test]
    fn tilde_f4() {
        let (ev, _) = parse(b"\x1b[14~").unwrap();
        assert_eq!(ev.key, Key::F(4));
    }

    #[test]
    fn tilde_f5() {
        let (ev, _) = parse(b"\x1b[15~").unwrap();
        assert_eq!(ev.key, Key::F(5));
    }

    #[test]
    fn tilde_f6() {
        let (ev, _) = parse(b"\x1b[17~").unwrap();
        assert_eq!(ev.key, Key::F(6));
    }

    #[test]
    fn tilde_f7() {
        let (ev, _) = parse(b"\x1b[18~").unwrap();
        assert_eq!(ev.key, Key::F(7));
    }

    #[test]
    fn tilde_f8() {
        let (ev, _) = parse(b"\x1b[19~").unwrap();
        assert_eq!(ev.key, Key::F(8));
    }

    #[test]
    fn tilde_f9() {
        let (ev, _) = parse(b"\x1b[20~").unwrap();
        assert_eq!(ev.key, Key::F(9));
    }

    #[test]
    fn tilde_f10() {
        let (ev, _) = parse(b"\x1b[21~").unwrap();
        assert_eq!(ev.key, Key::F(10));
    }

    #[test]
    fn tilde_f11() {
        let (ev, _) = parse(b"\x1b[23~").unwrap();
        assert_eq!(ev.key, Key::F(11));
    }

    #[test]
    fn tilde_f12() {
        let (ev, _) = parse(b"\x1b[24~").unwrap();
        assert_eq!(ev.key, Key::F(12));
    }

    // F13-F24 via Kitty codepoints.
    #[test]
    fn kitty_f13() {
        let (ev, _) = parse(b"\x1b[57364u").unwrap();
        assert_eq!(ev.key, Key::F(13));
    }

    #[test]
    fn kitty_f14() {
        let (ev, _) = parse(b"\x1b[57365u").unwrap();
        assert_eq!(ev.key, Key::F(14));
    }

    #[test]
    fn kitty_f20() {
        let (ev, _) = parse(b"\x1b[57371u").unwrap();
        assert_eq!(ev.key, Key::F(20));
    }

    #[test]
    fn kitty_f24() {
        let (ev, _) = parse(b"\x1b[57375u").unwrap();
        assert_eq!(ev.key, Key::F(24));
    }

    #[test]
    fn kitty_f35() {
        // F35 = 57364 + 35 - 13 = 57386
        let (ev, _) = parse(b"\x1b[57386u").unwrap();
        assert_eq!(ev.key, Key::F(35));
    }

    // F5 with modifiers (tilde encoding).
    #[test]
    fn f5_with_shift() {
        let (ev, _) = parse(b"\x1b[15;2~").unwrap();
        assert_eq!(ev.key, Key::F(5));
        assert!(ev.modifiers.contains(Modifiers::SHIFT));
    }

    #[test]
    fn f12_with_ctrl() {
        let (ev, _) = parse(b"\x1b[24;5~").unwrap();
        assert_eq!(ev.key, Key::F(12));
        assert!(ev.modifiers.contains(Modifiers::CTRL));
    }

    // ===================================================================
    // 10. Compose/IME sequences
    // ===================================================================

    #[test]
    fn compose_start_finish() {
        let mut cs = ComposeState::new();
        assert!(!cs.active);
        cs.start();
        assert!(cs.active);
        cs.feed('a');
        let result = cs.finish();
        assert_eq!(result, "a");
        assert!(!cs.active);
    }

    #[test]
    fn compose_cancel_clears() {
        let mut cs = ComposeState::new();
        cs.start();
        cs.feed('x');
        cs.feed('y');
        cs.cancel();
        assert!(!cs.active);
        assert!(cs.buffer.is_empty());
    }

    #[test]
    fn compose_multi_step() {
        let mut cs = ComposeState::new();
        cs.start();
        cs.feed('\'');
        cs.feed('e');
        let result = cs.finish();
        assert_eq!(result, "'e");
    }

    #[test]
    fn compose_with_combining_chars() {
        let mut cs = ComposeState::new();
        cs.start();
        cs.feed('a');
        cs.feed('\u{0308}'); // combining diaeresis
        let result = cs.finish();
        assert_eq!(result, "a\u{0308}");
    }

    #[test]
    fn compose_restart_after_finish() {
        let mut cs = ComposeState::new();
        cs.start();
        cs.feed('a');
        let _ = cs.finish();

        // Start new compose.
        cs.start();
        assert!(cs.buffer.is_empty());
        cs.feed('b');
        let result = cs.finish();
        assert_eq!(result, "b");
    }

    #[test]
    fn compose_restart_after_cancel() {
        let mut cs = ComposeState::new();
        cs.start();
        cs.feed('x');
        cs.cancel();

        cs.start();
        cs.feed('y');
        let result = cs.finish();
        assert_eq!(result, "y");
    }

    #[test]
    fn compose_default_trait() {
        let cs = ComposeState::default();
        assert!(!cs.active);
        assert!(cs.buffer.is_empty());
    }

    // ===================================================================
    // Additional: Alt+key combos
    // ===================================================================

    #[test]
    fn alt_enter() {
        let (ev, len) = parse(b"\x1b\r").unwrap();
        assert_eq!(ev.key, Key::Enter);
        assert!(ev.modifiers.contains(Modifiers::ALT));
        assert_eq!(len, 2);
    }

    #[test]
    fn alt_tab() {
        let (ev, len) = parse(b"\x1b\t").unwrap();
        assert_eq!(ev.key, Key::Tab);
        assert!(ev.modifiers.contains(Modifiers::ALT));
        assert_eq!(len, 2);
    }

    #[test]
    fn alt_number() {
        let (ev, _) = parse(b"\x1b1").unwrap();
        assert_eq!(ev.key, Key::Char('1'));
        assert!(ev.modifiers.contains(Modifiers::ALT));
    }

    #[test]
    fn alt_special_char() {
        let (ev, _) = parse(b"\x1b!").unwrap();
        assert_eq!(ev.key, Key::Char('!'));
        assert!(ev.modifiers.contains(Modifiers::ALT));
    }

    // ===================================================================
    // Additional: Display / formatting
    // ===================================================================

    #[test]
    fn key_display_all_variants() {
        assert_eq!(Key::Char('x').to_string(), "x");
        assert_eq!(Key::F(5).to_string(), "F5");
        assert_eq!(Key::Up.to_string(), "Up");
        assert_eq!(Key::Down.to_string(), "Down");
        assert_eq!(Key::Left.to_string(), "Left");
        assert_eq!(Key::Right.to_string(), "Right");
        assert_eq!(Key::Home.to_string(), "Home");
        assert_eq!(Key::End.to_string(), "End");
        assert_eq!(Key::PageUp.to_string(), "PageUp");
        assert_eq!(Key::PageDown.to_string(), "PageDown");
        assert_eq!(Key::Insert.to_string(), "Insert");
        assert_eq!(Key::Delete.to_string(), "Delete");
        assert_eq!(Key::Enter.to_string(), "Enter");
        assert_eq!(Key::Tab.to_string(), "Tab");
        assert_eq!(Key::Backspace.to_string(), "Backspace");
        assert_eq!(Key::Escape.to_string(), "Escape");
        assert_eq!(Key::CapsLock.to_string(), "CapsLock");
        assert_eq!(Key::ScrollLock.to_string(), "ScrollLock");
        assert_eq!(Key::NumLock.to_string(), "NumLock");
        assert_eq!(Key::PrintScreen.to_string(), "PrintScreen");
        assert_eq!(Key::Pause.to_string(), "Pause");
        assert_eq!(Key::Menu.to_string(), "Menu");
        assert_eq!(Key::Unknown(999).to_string(), "Unknown(999)");
    }

    #[test]
    fn key_event_display_all_modifiers() {
        let ev = KeyEvent::new(
            Key::Char('a'),
            Modifiers::CTRL | Modifiers::ALT | Modifiers::SHIFT | Modifiers::SUPER,
            EventType::Press,
        );
        assert_eq!(ev.to_string(), "Ctrl+Alt+Shift+Super+a");
    }

    #[test]
    fn key_event_display_super_key() {
        let ev = KeyEvent::new(Key::Char('s'), Modifiers::SUPER, EventType::Press);
        assert_eq!(ev.to_string(), "Super+s");
    }

    // ===================================================================
    // Additional: ParseError display
    // ===================================================================

    #[test]
    fn parse_error_display() {
        assert_eq!(ParseError::Empty.to_string(), "empty input");
        assert_eq!(
            ParseError::NotEscapeSequence.to_string(),
            "not an escape sequence"
        );
        assert_eq!(
            ParseError::MalformedParams.to_string(),
            "malformed CSI parameters"
        );
        assert_eq!(
            ParseError::UnknownFinal(0x5a).to_string(),
            "unknown CSI final byte: 0x5a"
        );
        assert_eq!(ParseError::Incomplete.to_string(), "incomplete sequence");
    }

    // ===================================================================
    // Additional: Ctrl+letter coverage
    // ===================================================================

    #[test]
    fn ctrl_b_through_y() {
        // Ctrl+B = 0x02, Ctrl+Y = 0x19
        for b in 0x02..=0x19_u8 {
            // Skip 0x09 (Tab), 0x0d (Enter)
            if b == 0x09 || b == 0x0d {
                continue;
            }
            let (ev, _) = parse(&[b]).unwrap();
            let expected_char = (b + b'a' - 1) as char;
            assert_eq!(ev.key, Key::Char(expected_char), "Ctrl+{expected_char}");
            assert!(ev.modifiers.contains(Modifiers::CTRL));
        }
    }

    // ===================================================================
    // Additional: Modifiers bitops
    // ===================================================================

    #[test]
    fn modifiers_bitand() {
        let m = Modifiers::CTRL | Modifiers::ALT;
        let masked = m & Modifiers::CTRL;
        assert!(masked.contains(Modifiers::CTRL));
        assert!(!masked.contains(Modifiers::ALT));
    }

    #[test]
    fn modifiers_bits_roundtrip() {
        let m = Modifiers::SHIFT | Modifiers::SUPER;
        assert_eq!(m.bits(), 0b0000_1001);
        let m2 = Modifiers::from_bits_truncate(m.bits());
        assert_eq!(m, m2);
    }

    // ===================================================================
    // Additional: SS3 unknown
    // ===================================================================

    #[test]
    fn ss3_unknown_final() {
        let result = parse(b"\x1bOA");
        assert_eq!(result, Err(ParseError::NotEscapeSequence));
    }
}
