//! ANSI escape sequence generation.
//!
//! Produces escape sequences as byte buffers (`Vec<u8>`) without
//! writing directly to any output stream.

use std::fmt::Write as _;

// ---------------------------------------------------------------------------
// Cursor movement
// ---------------------------------------------------------------------------

/// Move cursor to absolute position (1-based row and column).
#[must_use]
pub fn cursor_to(row: u16, col: u16) -> Vec<u8> {
    format!("\x1b[{row};{col}H").into_bytes()
}

/// Move cursor up by `n` rows.
#[must_use]
pub fn cursor_up(n: u16) -> Vec<u8> {
    format!("\x1b[{n}A").into_bytes()
}

/// Move cursor down by `n` rows.
#[must_use]
pub fn cursor_down(n: u16) -> Vec<u8> {
    format!("\x1b[{n}B").into_bytes()
}

/// Move cursor right by `n` columns.
#[must_use]
pub fn cursor_forward(n: u16) -> Vec<u8> {
    format!("\x1b[{n}C").into_bytes()
}

/// Move cursor left by `n` columns.
#[must_use]
pub fn cursor_back(n: u16) -> Vec<u8> {
    format!("\x1b[{n}D").into_bytes()
}

/// Move cursor to the beginning of the line `n` rows down.
#[must_use]
pub fn cursor_next_line(n: u16) -> Vec<u8> {
    format!("\x1b[{n}E").into_bytes()
}

/// Move cursor to the beginning of the line `n` rows up.
#[must_use]
pub fn cursor_prev_line(n: u16) -> Vec<u8> {
    format!("\x1b[{n}F").into_bytes()
}

/// Move cursor to an absolute column (1-based).
#[must_use]
pub fn cursor_to_column(col: u16) -> Vec<u8> {
    format!("\x1b[{col}G").into_bytes()
}

/// Hide the cursor.
#[must_use]
pub fn cursor_hide() -> Vec<u8> {
    b"\x1b[?25l".to_vec()
}

/// Show the cursor.
#[must_use]
pub fn cursor_show() -> Vec<u8> {
    b"\x1b[?25h".to_vec()
}

/// Save cursor position.
#[must_use]
pub fn cursor_save() -> Vec<u8> {
    b"\x1b[s".to_vec()
}

/// Restore cursor position.
#[must_use]
pub fn cursor_restore() -> Vec<u8> {
    b"\x1b[u".to_vec()
}

// ---------------------------------------------------------------------------
// Screen / line clear
// ---------------------------------------------------------------------------

/// Clear mode for erase operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClearMode {
    /// Clear from cursor to end.
    ToEnd,
    /// Clear from start to cursor.
    ToStart,
    /// Clear entire screen or line.
    All,
}

impl ClearMode {
    const fn param(self) -> u8 {
        match self {
            Self::ToEnd => 0,
            Self::ToStart => 1,
            Self::All => 2,
        }
    }
}

/// Clear part or all of the screen.
#[must_use]
pub fn clear_screen(mode: ClearMode) -> Vec<u8> {
    format!("\x1b[{}J", mode.param()).into_bytes()
}

/// Clear part or all of the current line.
#[must_use]
pub fn clear_line(mode: ClearMode) -> Vec<u8> {
    format!("\x1b[{}K", mode.param()).into_bytes()
}

// ---------------------------------------------------------------------------
// Color
// ---------------------------------------------------------------------------

/// A terminal color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    /// Standard ANSI color (0–7).
    Ansi(u8),
    /// Bright ANSI color (0–7, mapped to 8–15 in 256-color mode).
    AnsiBright(u8),
    /// 256-color palette index.
    Palette(u8),
    /// 24-bit true color.
    Rgb(u8, u8, u8),
}

// ---------------------------------------------------------------------------
// SGR style attributes
// ---------------------------------------------------------------------------

/// Text style attributes applied via SGR (Select Graphic Rendition).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct Style {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub blink: bool,
    pub reverse: bool,
    pub strikethrough: bool,
}

impl Style {
    /// Create a new default (unstyled) `Style`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            fg: None,
            bg: None,
            bold: false,
            dim: false,
            italic: false,
            underline: false,
            blink: false,
            reverse: false,
            strikethrough: false,
        }
    }

    /// Generate the SGR escape sequence for this style.
    ///
    /// Returns the full `\x1b[...m` sequence. If the style is completely
    /// default, returns a reset sequence.
    #[must_use]
    pub fn to_sgr(&self) -> Vec<u8> {
        if self == &Self::default() {
            return b"\x1b[0m".to_vec();
        }

        let mut buf = String::from("\x1b[0");

        if self.bold {
            buf.push_str(";1");
        }
        if self.dim {
            buf.push_str(";2");
        }
        if self.italic {
            buf.push_str(";3");
        }
        if self.underline {
            buf.push_str(";4");
        }
        if self.blink {
            buf.push_str(";5");
        }
        if self.reverse {
            buf.push_str(";7");
        }
        if self.strikethrough {
            buf.push_str(";9");
        }

        if let Some(color) = self.fg {
            write_color_sgr(&mut buf, color, false);
        }
        if let Some(color) = self.bg {
            write_color_sgr(&mut buf, color, true);
        }

        buf.push('m');
        buf.into_bytes()
    }
}

/// Append the SGR parameters for a color to a string buffer.
fn write_color_sgr(buf: &mut String, color: Color, is_bg: bool) {
    match color {
        Color::Ansi(n) => {
            let base: u8 = if is_bg { 40 } else { 30 };
            let _ = write!(buf, ";{}", base + n);
        }
        Color::AnsiBright(n) => {
            let base: u8 = if is_bg { 100 } else { 90 };
            let _ = write!(buf, ";{}", base + n);
        }
        Color::Palette(n) => {
            let prefix = if is_bg { "48" } else { "38" };
            let _ = write!(buf, ";{prefix};5;{n}");
        }
        Color::Rgb(r, g, b) => {
            let prefix = if is_bg { "48" } else { "38" };
            let _ = write!(buf, ";{prefix};2;{r};{g};{b}");
        }
    }
}

/// Generate a reset-all-attributes sequence.
#[must_use]
pub fn sgr_reset() -> Vec<u8> {
    b"\x1b[0m".to_vec()
}

// ---------------------------------------------------------------------------
// Styled cell and run-length encoding
// ---------------------------------------------------------------------------

/// A single cell with content and style.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyledCell {
    pub ch: char,
    pub style: Style,
}

/// Encode a row of styled cells into ANSI escape sequences.
///
/// Uses run-length encoding: consecutive cells with the same style
/// share a single SGR sequence, reducing output size.
#[must_use]
pub fn encode_row(cells: &[StyledCell]) -> Vec<u8> {
    if cells.is_empty() {
        return Vec::new();
    }

    let mut buf = Vec::new();
    let mut current_style: Option<&Style> = None;

    for cell in cells {
        if current_style != Some(&cell.style) {
            buf.extend_from_slice(&cell.style.to_sgr());
            current_style = Some(&cell.style);
        }
        let mut char_buf = [0u8; 4];
        let encoded = cell.ch.encode_utf8(&mut char_buf);
        buf.extend_from_slice(encoded.as_bytes());
    }

    // Reset style at the end of the row
    buf.extend_from_slice(&sgr_reset());
    buf
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Cursor movement --

    #[test]
    fn cursor_to_generates_cup() {
        assert_eq!(cursor_to(1, 1), b"\x1b[1;1H");
        assert_eq!(cursor_to(10, 20), b"\x1b[10;20H");
    }

    #[test]
    fn cursor_up_generates_cuu() {
        assert_eq!(cursor_up(1), b"\x1b[1A");
        assert_eq!(cursor_up(5), b"\x1b[5A");
    }

    #[test]
    fn cursor_down_generates_cud() {
        assert_eq!(cursor_down(1), b"\x1b[1B");
        assert_eq!(cursor_down(3), b"\x1b[3B");
    }

    #[test]
    fn cursor_forward_generates_cuf() {
        assert_eq!(cursor_forward(1), b"\x1b[1C");
        assert_eq!(cursor_forward(10), b"\x1b[10C");
    }

    #[test]
    fn cursor_back_generates_cub() {
        assert_eq!(cursor_back(1), b"\x1b[1D");
        assert_eq!(cursor_back(7), b"\x1b[7D");
    }

    #[test]
    fn cursor_next_line_generates_cnl() {
        assert_eq!(cursor_next_line(1), b"\x1b[1E");
    }

    #[test]
    fn cursor_prev_line_generates_cpl() {
        assert_eq!(cursor_prev_line(2), b"\x1b[2F");
    }

    #[test]
    fn cursor_to_column_generates_cha() {
        assert_eq!(cursor_to_column(5), b"\x1b[5G");
    }

    #[test]
    fn cursor_hide_and_show() {
        assert_eq!(cursor_hide(), b"\x1b[?25l");
        assert_eq!(cursor_show(), b"\x1b[?25h");
    }

    #[test]
    fn cursor_save_and_restore() {
        assert_eq!(cursor_save(), b"\x1b[s");
        assert_eq!(cursor_restore(), b"\x1b[u");
    }

    // -- Clear --

    #[test]
    fn clear_screen_modes() {
        assert_eq!(clear_screen(ClearMode::ToEnd), b"\x1b[0J");
        assert_eq!(clear_screen(ClearMode::ToStart), b"\x1b[1J");
        assert_eq!(clear_screen(ClearMode::All), b"\x1b[2J");
    }

    #[test]
    fn clear_line_modes() {
        assert_eq!(clear_line(ClearMode::ToEnd), b"\x1b[0K");
        assert_eq!(clear_line(ClearMode::ToStart), b"\x1b[1K");
        assert_eq!(clear_line(ClearMode::All), b"\x1b[2K");
    }

    // -- SGR / Style --

    #[test]
    fn default_style_is_reset() {
        assert_eq!(Style::new().to_sgr(), b"\x1b[0m");
    }

    #[test]
    fn bold_style() {
        let s = Style {
            bold: true,
            ..Style::new()
        };
        assert_eq!(s.to_sgr(), b"\x1b[0;1m");
    }

    #[test]
    fn dim_style() {
        let s = Style {
            dim: true,
            ..Style::new()
        };
        assert_eq!(s.to_sgr(), b"\x1b[0;2m");
    }

    #[test]
    fn italic_style() {
        let s = Style {
            italic: true,
            ..Style::new()
        };
        assert_eq!(s.to_sgr(), b"\x1b[0;3m");
    }

    #[test]
    fn underline_style() {
        let s = Style {
            underline: true,
            ..Style::new()
        };
        assert_eq!(s.to_sgr(), b"\x1b[0;4m");
    }

    #[test]
    fn blink_style() {
        let s = Style {
            blink: true,
            ..Style::new()
        };
        assert_eq!(s.to_sgr(), b"\x1b[0;5m");
    }

    #[test]
    fn reverse_style() {
        let s = Style {
            reverse: true,
            ..Style::new()
        };
        assert_eq!(s.to_sgr(), b"\x1b[0;7m");
    }

    #[test]
    fn strikethrough_style() {
        let s = Style {
            strikethrough: true,
            ..Style::new()
        };
        assert_eq!(s.to_sgr(), b"\x1b[0;9m");
    }

    #[test]
    fn combined_attributes() {
        let s = Style {
            bold: true,
            italic: true,
            underline: true,
            ..Style::new()
        };
        assert_eq!(s.to_sgr(), b"\x1b[0;1;3;4m");
    }

    #[test]
    fn fg_ansi_color() {
        let s = Style {
            fg: Some(Color::Ansi(1)), // Red
            ..Style::new()
        };
        assert_eq!(s.to_sgr(), b"\x1b[0;31m");
    }

    #[test]
    fn bg_ansi_color() {
        let s = Style {
            bg: Some(Color::Ansi(4)), // Blue
            ..Style::new()
        };
        assert_eq!(s.to_sgr(), b"\x1b[0;44m");
    }

    #[test]
    fn fg_bright_ansi_color() {
        let s = Style {
            fg: Some(Color::AnsiBright(2)), // Bright green
            ..Style::new()
        };
        assert_eq!(s.to_sgr(), b"\x1b[0;92m");
    }

    #[test]
    fn bg_bright_ansi_color() {
        let s = Style {
            bg: Some(Color::AnsiBright(5)),
            ..Style::new()
        };
        assert_eq!(s.to_sgr(), b"\x1b[0;105m");
    }

    #[test]
    fn fg_palette_color() {
        let s = Style {
            fg: Some(Color::Palette(196)),
            ..Style::new()
        };
        assert_eq!(s.to_sgr(), b"\x1b[0;38;5;196m");
    }

    #[test]
    fn bg_palette_color() {
        let s = Style {
            bg: Some(Color::Palette(33)),
            ..Style::new()
        };
        assert_eq!(s.to_sgr(), b"\x1b[0;48;5;33m");
    }

    #[test]
    fn fg_rgb_true_color() {
        let s = Style {
            fg: Some(Color::Rgb(255, 128, 0)),
            ..Style::new()
        };
        assert_eq!(s.to_sgr(), b"\x1b[0;38;2;255;128;0m");
    }

    #[test]
    fn bg_rgb_true_color() {
        let s = Style {
            bg: Some(Color::Rgb(0, 0, 0)),
            ..Style::new()
        };
        assert_eq!(s.to_sgr(), b"\x1b[0;48;2;0;0;0m");
    }

    #[test]
    fn fg_and_bg_combined() {
        let s = Style {
            fg: Some(Color::Rgb(255, 255, 255)),
            bg: Some(Color::Rgb(0, 0, 0)),
            bold: true,
            ..Style::new()
        };
        assert_eq!(s.to_sgr(), b"\x1b[0;1;38;2;255;255;255;48;2;0;0;0m");
    }

    #[test]
    fn sgr_reset_sequence() {
        assert_eq!(sgr_reset(), b"\x1b[0m");
    }

    // -- Run-length encoding --

    #[test]
    fn encode_empty_row() {
        assert_eq!(encode_row(&[]), Vec::<u8>::new());
    }

    #[test]
    fn encode_single_cell() {
        let cells = vec![StyledCell {
            ch: 'A',
            style: Style::new(),
        }];
        let result = encode_row(&cells);
        // Reset style + 'A' + trailing reset
        assert_eq!(result, b"\x1b[0mA\x1b[0m");
    }

    #[test]
    fn encode_same_style_run() {
        let style = Style {
            bold: true,
            ..Style::new()
        };
        let cells = vec![
            StyledCell { ch: 'H', style },
            StyledCell { ch: 'i', style },
            StyledCell { ch: '!', style },
        ];
        let result = encode_row(&cells);
        // One SGR for the whole run, then trailing reset
        assert_eq!(result, b"\x1b[0;1mHi!\x1b[0m");
    }

    #[test]
    fn encode_style_change_mid_row() {
        let plain = Style::new();
        let bold = Style {
            bold: true,
            ..Style::new()
        };
        let cells = vec![
            StyledCell {
                ch: 'A',
                style: plain,
            },
            StyledCell {
                ch: 'B',
                style: bold,
            },
            StyledCell {
                ch: 'C',
                style: bold,
            },
            StyledCell {
                ch: 'D',
                style: plain,
            },
        ];
        let result = encode_row(&cells);
        let expected = b"\x1b[0mA\x1b[0;1mBC\x1b[0mD\x1b[0m";
        assert_eq!(result, expected.to_vec());
    }

    #[test]
    fn encode_row_with_colors() {
        let red_on_black = Style {
            fg: Some(Color::Rgb(255, 0, 0)),
            bg: Some(Color::Rgb(0, 0, 0)),
            ..Style::new()
        };
        let cells = vec![
            StyledCell {
                ch: 'X',
                style: red_on_black,
            },
            StyledCell {
                ch: 'Y',
                style: red_on_black,
            },
        ];
        let result = encode_row(&cells);
        assert_eq!(result, b"\x1b[0;38;2;255;0;0;48;2;0;0;0mXY\x1b[0m");
    }

    #[test]
    fn encode_row_unicode() {
        let style = Style::new();
        let cells = vec![
            StyledCell { ch: '🐱', style },
            StyledCell { ch: '💻', style },
        ];
        let result = encode_row(&cells);
        let result_str =
            std::str::from_utf8(&result).unwrap_or_else(|_| panic!("output should be valid utf-8"));
        assert!(result_str.contains("🐱"));
        assert!(result_str.contains("💻"));
    }

    #[test]
    fn encode_alternating_styles() {
        let a = Style {
            bold: true,
            ..Style::new()
        };
        let b = Style {
            italic: true,
            ..Style::new()
        };
        let cells = vec![
            StyledCell { ch: '1', style: a },
            StyledCell { ch: '2', style: b },
            StyledCell { ch: '3', style: a },
            StyledCell { ch: '4', style: b },
        ];
        let result = encode_row(&cells);
        // Each change emits a new SGR
        let result_str =
            std::str::from_utf8(&result).unwrap_or_else(|_| panic!("output should be valid utf-8"));
        // 4 style switches + trailing reset = 5 escape sequences
        assert_eq!(result_str.matches("\x1b[").count(), 5);
    }
}
