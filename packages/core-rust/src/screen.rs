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
    let mut stdout = io::stdout().lock();
    stdout.write_all(ENTER_ALT_SCREEN)?;
    stdout.flush()
}

/// Exit the alternate screen buffer and restore the main screen.
///
/// # Errors
///
/// Returns an error if writing to stdout fails.
pub fn exit() -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    stdout.write_all(EXIT_ALT_SCREEN)?;
    stdout.flush()
}
