//! Buffered stdout writer with explicit flush control.
//!
//! All terminal output goes through this writer so we can batch escape
//! sequences and minimise the number of `write` syscalls.

use std::io::{self, BufWriter, Write};

/// Default buffer capacity in bytes (8 KiB).
const DEFAULT_CAPACITY: usize = 8 * 1024;

/// A buffered writer wrapping stdout.
pub struct TermWriter {
    inner: BufWriter<io::Stdout>,
}

impl TermWriter {
    /// Create a new `TermWriter` with the default buffer capacity.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: BufWriter::with_capacity(DEFAULT_CAPACITY, io::stdout()),
        }
    }

    /// Create a new `TermWriter` with a custom buffer capacity.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: BufWriter::with_capacity(capacity, io::stdout()),
        }
    }

    /// Write bytes into the buffer without flushing.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying write fails.
    pub fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.inner.write(data)
    }

    /// Write a string slice into the buffer.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying write fails.
    pub fn write_str(&mut self, s: &str) -> io::Result<usize> {
        self.inner.write(s.as_bytes())
    }

    /// Flush all buffered data to stdout.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying flush fails.
    pub fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }

    /// Returns the current number of buffered bytes.
    #[must_use]
    pub fn buffered_len(&self) -> usize {
        self.inner.buffer().len()
    }
}

impl Default for TermWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_writer_has_empty_buffer() {
        let writer = TermWriter::new();
        assert_eq!(writer.buffered_len(), 0);
    }

    #[test]
    fn write_increases_buffered_len() {
        let mut writer = TermWriter::new();
        let data = b"hello terminal";
        writer.write(data).unwrap();
        assert_eq!(writer.buffered_len(), data.len());
    }

    #[test]
    fn write_str_increases_buffered_len() {
        let mut writer = TermWriter::new();
        let s = "escape \x1b[2J";
        writer.write_str(s).unwrap();
        assert_eq!(writer.buffered_len(), s.len());
    }

    #[test]
    fn flush_empties_buffer() {
        let mut writer = TermWriter::new();
        writer.write(b"data").unwrap();
        assert!(writer.buffered_len() > 0);
        writer.flush().unwrap();
        assert_eq!(writer.buffered_len(), 0);
    }

    #[test]
    fn custom_capacity_works() {
        let writer = TermWriter::with_capacity(256);
        assert_eq!(writer.buffered_len(), 0);
    }
}
