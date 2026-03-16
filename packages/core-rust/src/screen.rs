//! Alternate screen buffer management.
//!
//! Terminal applications switch to the alternate screen so they don't
//! clobber the user's scrollback when they exit.

use std::io::{self, Write};

/// ANSI escape to enter the alternate screen buffer.
const ENTER_ALT_SCREEN: &[u8] = b"\x1b[?1049h";

/// ANSI escape to leave the alternate screen buffer.
const EXIT_ALT_SCREEN: &[u8] = b"\x1b[?1049l";

/// Enter the alternate screen buffer.
///
/// # Errors
///
/// Returns an error if writing to stdout fails.
pub fn enter() -> io::Result<()> {
    enter_to(&mut io::stdout().lock())
}

/// Exit the alternate screen buffer and restore the main screen.
///
/// # Errors
///
/// Returns an error if writing to stdout fails.
pub fn exit() -> io::Result<()> {
    exit_to(&mut io::stdout().lock())
}

/// Write the enter-alternate-screen escape to any writer.
///
/// # Errors
///
/// Returns an error if writing fails.
pub fn enter_to<W: Write>(w: &mut W) -> io::Result<()> {
    w.write_all(ENTER_ALT_SCREEN)?;
    w.flush()
}

/// Write the exit-alternate-screen escape to any writer.
///
/// # Errors
///
/// Returns an error if writing fails.
pub fn exit_to<W: Write>(w: &mut W) -> io::Result<()> {
    w.write_all(EXIT_ALT_SCREEN)?;
    w.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enter_writes_correct_escape() {
        let mut buf = Vec::new();
        enter_to(&mut buf).unwrap();
        assert_eq!(buf, b"\x1b[?1049h");
    }

    #[test]
    fn exit_writes_correct_escape() {
        let mut buf = Vec::new();
        exit_to(&mut buf).unwrap();
        assert_eq!(buf, b"\x1b[?1049l");
    }

    #[test]
    fn enter_then_exit_produces_both_escapes() {
        let mut buf = Vec::new();
        enter_to(&mut buf).unwrap();
        exit_to(&mut buf).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("\x1b[?1049h"));
        assert!(output.contains("\x1b[?1049l"));
    }

    #[test]
    fn enter_and_exit_escapes_are_distinct() {
        assert_ne!(ENTER_ALT_SCREEN, EXIT_ALT_SCREEN);
    }
}
