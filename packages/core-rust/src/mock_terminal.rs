//! Mock terminal backend for headless testing.
//!
//! Captures all terminal output in memory so tests can assert on
//! escape sequences and rendered content without a real TTY.

use std::io::{self, Write};

/// A mock terminal that captures output in an in-memory buffer.
///
/// Use this in tests instead of writing to stdout so tests can run
/// in CI without a TTY.
pub struct MockTerminal {
    output: Vec<u8>,
    width: u16,
    height: u16,
}

impl MockTerminal {
    /// Create a new mock terminal with the given dimensions.
    #[must_use]
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            output: Vec::new(),
            width,
            height,
        }
    }

    /// Returns the raw bytes written to this terminal.
    #[must_use]
    pub fn output(&self) -> &[u8] {
        &self.output
    }

    /// Returns the output as a UTF-8 string.
    ///
    /// # Panics
    ///
    /// Panics if the output is not valid UTF-8.
    #[must_use]
    pub fn output_str(&self) -> &str {
        std::str::from_utf8(&self.output)
            .unwrap_or_else(|_| panic!("MockTerminal output is not valid UTF-8"))
    }

    /// Returns the terminal width.
    #[must_use]
    pub fn width(&self) -> u16 {
        self.width
    }

    /// Returns the terminal height.
    #[must_use]
    pub fn height(&self) -> u16 {
        self.height
    }

    /// Clear all captured output.
    pub fn clear(&mut self) {
        self.output.clear();
    }

    /// Check whether the output contains a given byte sequence.
    #[must_use]
    pub fn output_contains(&self, needle: &[u8]) -> bool {
        self.output
            .windows(needle.len())
            .any(|window| window == needle)
    }

    /// Check whether the output contains a given string.
    #[must_use]
    pub fn output_contains_str(&self, needle: &str) -> bool {
        self.output_contains(needle.as_bytes())
    }
}

impl Write for MockTerminal {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.output.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_terminal_has_empty_output() {
        let term = MockTerminal::new(80, 24);
        assert!(term.output().is_empty());
        assert_eq!(term.width(), 80);
        assert_eq!(term.height(), 24);
    }

    #[test]
    fn write_captures_output() {
        let mut term = MockTerminal::new(80, 24);
        term.write_all(b"Hello").unwrap();
        assert_eq!(term.output(), b"Hello");
        assert_eq!(term.output_str(), "Hello");
    }

    #[test]
    fn write_captures_escape_sequences() {
        let mut term = MockTerminal::new(80, 24);
        term.write_all(b"\x1b[?1049h").unwrap();
        assert!(term.output_contains(b"\x1b[?1049h"));
        assert!(term.output_contains_str("\x1b[?1049h"));
    }

    #[test]
    fn clear_resets_output() {
        let mut term = MockTerminal::new(80, 24);
        term.write_all(b"data").unwrap();
        assert!(!term.output().is_empty());
        term.clear();
        assert!(term.output().is_empty());
    }

    #[test]
    fn multiple_writes_accumulate() {
        let mut term = MockTerminal::new(80, 24);
        term.write_all(b"first").unwrap();
        term.write_all(b"second").unwrap();
        assert_eq!(term.output_str(), "firstsecond");
    }

    #[test]
    fn output_contains_returns_false_for_missing() {
        let term = MockTerminal::new(80, 24);
        assert!(!term.output_contains(b"missing"));
        assert!(!term.output_contains_str("missing"));
    }

    #[test]
    fn flush_is_noop() {
        let mut term = MockTerminal::new(80, 24);
        term.write_all(b"data").unwrap();
        term.flush().unwrap();
        assert_eq!(term.output_str(), "data");
    }
}
