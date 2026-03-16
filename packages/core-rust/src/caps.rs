//! Terminal capability detection.
//!
//! Queries the terminal for Kitty protocol support, keyboard protocol,
//! pixel-precise mouse, and cell dimensions. Results are collected into
//! a [`TerminalCaps`] struct.

use std::io::{self, Read, Write};

// ---------------------------------------------------------------------------
// Query / response escape sequences
// ---------------------------------------------------------------------------

/// Kitty graphics protocol query: send a 1x1 transparent pixel query image.
/// A Kitty-compatible terminal responds with `\x1b_Gi=31;OK\x1b\\`.
const KITTY_GRAPHICS_QUERY: &[u8] = b"\x1b_Gi=31,s=1,v=1,a=q,t=d,f=24;AAAA\x1b\\";

/// Kitty keyboard protocol query: request current keyboard mode.
/// A supporting terminal responds with `\x1b[?{mode}u`.
const KITTY_KEYBOARD_QUERY: &[u8] = b"\x1b[?u";

/// Query cell size in pixels via mode 14t.
/// Terminal responds with `\x1b[4;{height};{width}t`.
const CELL_SIZE_QUERY: &[u8] = b"\x1b[16t";

/// Primary Device Attributes query — used as a synchronization fence.
/// Every terminal responds with `\x1b[?...c`, so we use it to detect
/// when the terminal has finished answering our queries.
const DA1_QUERY: &[u8] = b"\x1b[c";

/// SGR-pixel mouse mode probe: attempt to enable, then check.
/// If the terminal supports pixel mouse, it acknowledges mode 1016.
const SGR_PIXEL_MOUSE_ENABLE: &[u8] = b"\x1b[?1016h";

/// Disable SGR-pixel mouse after probing.
const SGR_PIXEL_MOUSE_DISABLE: &[u8] = b"\x1b[?1016l";

/// DECRPM (DEC Private Mode Report) query for mode 1016.
/// Response: `\x1b[?1016;{Ps}$y` where Ps=1 means set (supported),
/// Ps=2 means reset, Ps=0 means not recognized.
const SGR_PIXEL_MOUSE_QUERY: &[u8] = b"\x1b[?1016$p";

// ---------------------------------------------------------------------------
// TerminalCaps
// ---------------------------------------------------------------------------

/// Detected terminal capabilities.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TerminalCaps {
    /// Whether the terminal supports the Kitty graphics protocol.
    pub kitty_graphics: bool,
    /// Whether the terminal supports the Kitty keyboard protocol.
    pub kitty_keyboard: bool,
    /// Whether the terminal supports SGR-pixel mouse reporting.
    pub pixel_mouse: bool,
    /// Cell width in pixels, if detected.
    pub cell_width_px: Option<u16>,
    /// Cell height in pixels, if detected.
    pub cell_height_px: Option<u16>,
}

// ---------------------------------------------------------------------------
// TerminalQuerier trait — abstracts query/response for testability
// ---------------------------------------------------------------------------

/// Trait for sending queries to a terminal and reading responses.
///
/// Implement this for real terminal I/O or for a mock test backend.
pub trait TerminalQuerier {
    /// Send a query sequence to the terminal.
    ///
    /// # Errors
    ///
    /// Returns an error if writing fails.
    fn send_query(&mut self, query: &[u8]) -> io::Result<()>;

    /// Read the terminal's response into the provided buffer.
    ///
    /// Returns the number of bytes read. Implementations should have
    /// a timeout to avoid blocking forever if the terminal doesn't respond.
    ///
    /// # Errors
    ///
    /// Returns an error if reading fails.
    fn read_response(&mut self, buf: &mut [u8]) -> io::Result<usize>;
}

// ---------------------------------------------------------------------------
// Response parsers
// ---------------------------------------------------------------------------

/// Check if a response buffer contains a successful Kitty graphics reply.
///
/// The terminal responds with `\x1b_Gi=31;OK\x1b\\` on success.
#[must_use]
pub fn parse_kitty_graphics_response(response: &[u8]) -> bool {
    contains_subsequence(response, b"\x1b_Gi=31;OK\x1b\\")
}

/// Check if a response buffer contains a Kitty keyboard protocol reply.
///
/// The terminal responds with `\x1b[?{mode}u` where mode is a digit.
#[must_use]
pub fn parse_kitty_keyboard_response(response: &[u8]) -> bool {
    // Look for \x1b[?<digits>u
    let mut i = 0;
    while i < response.len() {
        if response[i] == 0x1b
            && i + 2 < response.len()
            && response[i + 1] == b'['
            && response[i + 2] == b'?'
        {
            let start = i + 3;
            let mut j = start;
            while j < response.len() && response[j].is_ascii_digit() {
                j += 1;
            }
            if j > start && j < response.len() && response[j] == b'u' {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// Parse cell size from a terminal response.
///
/// Looks for `\x1b[6;{height};{width}t` (response to mode 16t query).
#[must_use]
pub fn parse_cell_size_response(response: &[u8]) -> Option<(u16, u16)> {
    // Look for \x1b[6;<height>;<width>t
    let mut i = 0;
    while i < response.len() {
        if response[i] == 0x1b && i + 1 < response.len() && response[i + 1] == b'[' {
            let after_csi = i + 2;
            if let Some((height, width)) = parse_cell_size_params(&response[after_csi..]) {
                return Some((height, width));
            }
        }
        i += 1;
    }
    None
}

/// Parse `6;<height>;<width>t` from a byte slice starting after `\x1b[`.
fn parse_cell_size_params(data: &[u8]) -> Option<(u16, u16)> {
    // Expect "6;" prefix
    if data.len() < 4 || data[0] != b'6' || data[1] != b';' {
        return None;
    }

    let rest = &data[2..];
    let (height, consumed) = parse_u16(rest)?;
    if consumed >= rest.len() || rest[consumed] != b';' {
        return None;
    }
    let rest = &rest[consumed + 1..];
    let (width, consumed) = parse_u16(rest)?;
    if consumed >= rest.len() || rest[consumed] != b't' {
        return None;
    }

    Some((height, width))
}

/// Check if a response indicates SGR-pixel mouse support.
///
/// Looks for DECRPM response `\x1b[?1016;1$y` (mode set) or
/// `\x1b[?1016;2$y` (mode reset — still means recognized).
/// Ps=0 means not recognized (not supported).
#[must_use]
pub fn parse_pixel_mouse_response(response: &[u8]) -> bool {
    // Look for \x1b[?1016;{Ps}$y where Ps is 1 or 2
    contains_subsequence(response, b"\x1b[?1016;1$y")
        || contains_subsequence(response, b"\x1b[?1016;2$y")
}

// ---------------------------------------------------------------------------
// Query builder
// ---------------------------------------------------------------------------

/// Build the complete query sequence to detect all capabilities.
///
/// Sends all probe queries followed by a DA1 fence, so we know when
/// to stop reading responses.
#[must_use]
pub fn build_caps_query() -> Vec<u8> {
    let mut query = Vec::new();
    query.extend_from_slice(KITTY_GRAPHICS_QUERY);
    query.extend_from_slice(KITTY_KEYBOARD_QUERY);
    query.extend_from_slice(CELL_SIZE_QUERY);
    query.extend_from_slice(SGR_PIXEL_MOUSE_ENABLE);
    query.extend_from_slice(SGR_PIXEL_MOUSE_QUERY);
    query.extend_from_slice(SGR_PIXEL_MOUSE_DISABLE);
    query.extend_from_slice(DA1_QUERY);
    query
}

/// Detect terminal capabilities using the provided querier.
///
/// # Errors
///
/// Returns an error if communication with the terminal fails.
pub fn detect(querier: &mut dyn TerminalQuerier) -> io::Result<TerminalCaps> {
    let query = build_caps_query();
    querier.send_query(&query)?;

    // Read response — we keep reading until we see the DA1 response
    // (\x1b[?...c) or we fill our buffer.
    let mut response = Vec::new();
    let mut buf = [0u8; 256];

    for _ in 0..20 {
        let n = querier.read_response(&mut buf)?;
        if n == 0 {
            break;
        }
        response.extend_from_slice(&buf[..n]);

        // DA1 response ends with 'c' — check if we've received it.
        if response
            .windows(2)
            .any(|w| w[0] == b'?' && response.last() == Some(&b'c'))
            || contains_subsequence(&response, b"\x1b[?") && response.contains(&b'c')
        {
            // Check if there's a complete DA1 response
            if has_da1_response(&response) {
                break;
            }
        }
    }

    Ok(parse_all_responses(&response))
}

/// Parse all capability responses from a combined response buffer.
#[must_use]
pub fn parse_all_responses(response: &[u8]) -> TerminalCaps {
    let cell_size = parse_cell_size_response(response);

    TerminalCaps {
        kitty_graphics: parse_kitty_graphics_response(response),
        kitty_keyboard: parse_kitty_keyboard_response(response),
        pixel_mouse: parse_pixel_mouse_response(response),
        cell_width_px: cell_size.map(|(_, w)| w),
        cell_height_px: cell_size.map(|(h, _)| h),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Check if a DA1 response is present in the buffer.
fn has_da1_response(data: &[u8]) -> bool {
    // DA1 response: \x1b[?<params>c
    let mut i = 0;
    while i < data.len() {
        if data[i] == 0x1b && i + 2 < data.len() && data[i + 1] == b'[' && data[i + 2] == b'?' {
            let start = i + 3;
            let mut j = start;
            while j < data.len() && data[j] != b'c' {
                j += 1;
            }
            if j < data.len() && data[j] == b'c' && j > start {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// Check if `haystack` contains `needle` as a contiguous subsequence.
fn contains_subsequence(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

/// Parse a decimal u16 from the start of a byte slice.
/// Returns the value and the number of bytes consumed.
fn parse_u16(data: &[u8]) -> Option<(u16, usize)> {
    let mut val: u16 = 0;
    let mut count = 0;
    for &b in data {
        if b.is_ascii_digit() {
            val = val.checked_mul(10)?.checked_add(u16::from(b - b'0'))?;
            count += 1;
        } else {
            break;
        }
    }
    if count == 0 {
        None
    } else {
        Some((val, count))
    }
}

// ---------------------------------------------------------------------------
// Mock querier for testing
// ---------------------------------------------------------------------------

/// A mock querier that returns a pre-configured response.
///
/// Useful for testing the capability detection logic without a real terminal.
pub struct MockQuerier {
    response: Vec<u8>,
    sent: Vec<u8>,
    read_offset: usize,
}

impl MockQuerier {
    /// Create a mock querier that will return `response` when read.
    #[must_use]
    pub fn new(response: Vec<u8>) -> Self {
        Self {
            response,
            sent: Vec::new(),
            read_offset: 0,
        }
    }

    /// Returns what was sent to this mock querier.
    #[must_use]
    pub fn sent(&self) -> &[u8] {
        &self.sent
    }
}

impl TerminalQuerier for MockQuerier {
    fn send_query(&mut self, query: &[u8]) -> io::Result<()> {
        self.sent.extend_from_slice(query);
        Ok(())
    }

    fn read_response(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.read_offset >= self.response.len() {
            return Ok(0);
        }
        let remaining = &self.response[self.read_offset..];
        let n = remaining.len().min(buf.len());
        buf[..n].copy_from_slice(&remaining[..n]);
        self.read_offset += n;
        Ok(n)
    }
}

/// A real terminal querier that reads/writes via provided streams.
pub struct StdioQuerier<W: Write, R: Read> {
    writer: W,
    reader: R,
}

impl<W: Write, R: Read> StdioQuerier<W, R> {
    /// Create a new querier with the given writer and reader.
    pub fn new(writer: W, reader: R) -> Self {
        Self { writer, reader }
    }
}

impl<W: Write, R: Read> TerminalQuerier for StdioQuerier<W, R> {
    fn send_query(&mut self, query: &[u8]) -> io::Result<()> {
        self.writer.write_all(query)?;
        self.writer.flush()
    }

    fn read_response(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.reader.read(buf)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Query constants --

    #[test]
    fn kitty_graphics_query_is_valid_apc() {
        // APC starts with \x1b_ and ends with ST (\x1b\\)
        assert!(KITTY_GRAPHICS_QUERY.starts_with(b"\x1b_"));
        assert!(KITTY_GRAPHICS_QUERY.ends_with(b"\x1b\\"));
    }

    #[test]
    fn build_caps_query_includes_all_probes() {
        let query = build_caps_query();
        assert!(contains_subsequence(&query, KITTY_GRAPHICS_QUERY));
        assert!(contains_subsequence(&query, KITTY_KEYBOARD_QUERY));
        assert!(contains_subsequence(&query, CELL_SIZE_QUERY));
        assert!(contains_subsequence(&query, SGR_PIXEL_MOUSE_QUERY));
        assert!(contains_subsequence(&query, DA1_QUERY));
    }

    #[test]
    fn build_caps_query_ends_with_da1_fence() {
        let query = build_caps_query();
        assert!(query.ends_with(DA1_QUERY));
    }

    // -- Kitty graphics parsing --

    #[test]
    fn parse_kitty_graphics_positive() {
        let response = b"\x1b_Gi=31;OK\x1b\\";
        assert!(parse_kitty_graphics_response(response));
    }

    #[test]
    fn parse_kitty_graphics_negative() {
        let response = b"\x1b_Gi=31;ENOENT\x1b\\";
        assert!(!parse_kitty_graphics_response(response));
    }

    #[test]
    fn parse_kitty_graphics_empty() {
        assert!(!parse_kitty_graphics_response(b""));
    }

    #[test]
    fn parse_kitty_graphics_in_mixed_response() {
        let mut response = Vec::new();
        response.extend_from_slice(b"\x1b[?65;1;9c"); // DA1
        response.extend_from_slice(b"\x1b_Gi=31;OK\x1b\\"); // Graphics OK
        assert!(parse_kitty_graphics_response(&response));
    }

    // -- Kitty keyboard parsing --

    #[test]
    fn parse_kitty_keyboard_positive() {
        let response = b"\x1b[?1u";
        assert!(parse_kitty_keyboard_response(response));
    }

    #[test]
    fn parse_kitty_keyboard_mode_zero() {
        let response = b"\x1b[?0u";
        assert!(parse_kitty_keyboard_response(response));
    }

    #[test]
    fn parse_kitty_keyboard_negative() {
        assert!(!parse_kitty_keyboard_response(b""));
        assert!(!parse_kitty_keyboard_response(b"\x1b[?65c"));
    }

    // -- Cell size parsing --

    #[test]
    fn parse_cell_size_positive() {
        let response = b"\x1b[6;16;8t";
        assert_eq!(parse_cell_size_response(response), Some((16, 8)));
    }

    #[test]
    fn parse_cell_size_large_values() {
        let response = b"\x1b[6;32;14t";
        assert_eq!(parse_cell_size_response(response), Some((32, 14)));
    }

    #[test]
    fn parse_cell_size_negative() {
        assert_eq!(parse_cell_size_response(b""), None);
        assert_eq!(parse_cell_size_response(b"\x1b[6;t"), None);
    }

    #[test]
    fn parse_cell_size_in_mixed_response() {
        let mut response = Vec::new();
        response.extend_from_slice(b"\x1b[?1u");
        response.extend_from_slice(b"\x1b[6;20;10t");
        response.extend_from_slice(b"\x1b[?65c");
        assert_eq!(parse_cell_size_response(&response), Some((20, 10)));
    }

    // -- Pixel mouse parsing --

    #[test]
    fn parse_pixel_mouse_supported_set() {
        let response = b"\x1b[?1016;1$y";
        assert!(parse_pixel_mouse_response(response));
    }

    #[test]
    fn parse_pixel_mouse_supported_reset() {
        let response = b"\x1b[?1016;2$y";
        assert!(parse_pixel_mouse_response(response));
    }

    #[test]
    fn parse_pixel_mouse_not_recognized() {
        let response = b"\x1b[?1016;0$y";
        assert!(!parse_pixel_mouse_response(response));
    }

    #[test]
    fn parse_pixel_mouse_empty() {
        assert!(!parse_pixel_mouse_response(b""));
    }

    // -- Combined parsing --

    #[test]
    fn parse_all_responses_full_support() {
        let mut response = Vec::new();
        response.extend_from_slice(b"\x1b_Gi=31;OK\x1b\\");
        response.extend_from_slice(b"\x1b[?1u");
        response.extend_from_slice(b"\x1b[6;16;8t");
        response.extend_from_slice(b"\x1b[?1016;1$y");
        response.extend_from_slice(b"\x1b[?65;1c");

        let caps = parse_all_responses(&response);
        assert!(caps.kitty_graphics);
        assert!(caps.kitty_keyboard);
        assert!(caps.pixel_mouse);
        assert_eq!(caps.cell_width_px, Some(8));
        assert_eq!(caps.cell_height_px, Some(16));
    }

    #[test]
    fn parse_all_responses_no_support() {
        // Only DA1 response — no protocol support
        let response = b"\x1b[?65;1c";
        let caps = parse_all_responses(response);
        assert!(!caps.kitty_graphics);
        assert!(!caps.kitty_keyboard);
        assert!(!caps.pixel_mouse);
        assert_eq!(caps.cell_width_px, None);
        assert_eq!(caps.cell_height_px, None);
    }

    #[test]
    fn parse_all_responses_partial_support() {
        let mut response = Vec::new();
        response.extend_from_slice(b"\x1b_Gi=31;OK\x1b\\");
        // No keyboard or mouse or cell size
        response.extend_from_slice(b"\x1b[?65;1c");

        let caps = parse_all_responses(&response);
        assert!(caps.kitty_graphics);
        assert!(!caps.kitty_keyboard);
        assert!(!caps.pixel_mouse);
        assert_eq!(caps.cell_width_px, None);
    }

    // -- MockQuerier --

    #[test]
    fn mock_querier_records_sent_queries() {
        let mut mock = MockQuerier::new(Vec::new());
        mock.send_query(b"hello").unwrap();
        assert_eq!(mock.sent(), b"hello");
    }

    #[test]
    fn mock_querier_returns_configured_response() {
        let mut mock = MockQuerier::new(b"response".to_vec());
        let mut buf = [0u8; 64];
        let n = mock.read_response(&mut buf).unwrap();
        assert_eq!(&buf[..n], b"response");
    }

    #[test]
    fn mock_querier_returns_zero_when_exhausted() {
        let mut mock = MockQuerier::new(b"x".to_vec());
        let mut buf = [0u8; 64];
        let _ = mock.read_response(&mut buf).unwrap();
        let n = mock.read_response(&mut buf).unwrap();
        assert_eq!(n, 0);
    }

    // -- detect() with MockQuerier --

    #[test]
    fn detect_with_full_kitty_support() {
        let mut response = Vec::new();
        response.extend_from_slice(b"\x1b_Gi=31;OK\x1b\\");
        response.extend_from_slice(b"\x1b[?1u");
        response.extend_from_slice(b"\x1b[6;18;9t");
        response.extend_from_slice(b"\x1b[?1016;1$y");
        response.extend_from_slice(b"\x1b[?65;1;9c");

        let mut mock = MockQuerier::new(response);
        let caps = detect(&mut mock).unwrap();

        assert!(caps.kitty_graphics);
        assert!(caps.kitty_keyboard);
        assert!(caps.pixel_mouse);
        assert_eq!(caps.cell_width_px, Some(9));
        assert_eq!(caps.cell_height_px, Some(18));

        // Verify the query was actually sent
        assert!(!mock.sent().is_empty());
    }

    #[test]
    fn detect_with_no_support() {
        let response = b"\x1b[?65;1c".to_vec();
        let mut mock = MockQuerier::new(response);
        let caps = detect(&mut mock).unwrap();

        assert!(!caps.kitty_graphics);
        assert!(!caps.kitty_keyboard);
        assert!(!caps.pixel_mouse);
        assert_eq!(caps.cell_width_px, None);
        assert_eq!(caps.cell_height_px, None);
    }

    #[test]
    fn detect_with_empty_response() {
        let mut mock = MockQuerier::new(Vec::new());
        let caps = detect(&mut mock).unwrap();
        assert_eq!(caps, TerminalCaps::default());
    }

    // -- Helper tests --

    #[test]
    fn contains_subsequence_basic() {
        assert!(contains_subsequence(b"hello world", b"world"));
        assert!(!contains_subsequence(b"hello", b"world"));
        assert!(contains_subsequence(b"abc", b""));
        assert!(!contains_subsequence(b"", b"a"));
    }

    #[test]
    fn parse_u16_basic() {
        assert_eq!(parse_u16(b"123abc"), Some((123, 3)));
        assert_eq!(parse_u16(b"0"), Some((0, 1)));
        assert_eq!(parse_u16(b"65535"), Some((65535, 5)));
        assert_eq!(parse_u16(b"abc"), None);
        assert_eq!(parse_u16(b""), None);
    }

    #[test]
    fn has_da1_response_positive() {
        assert!(has_da1_response(b"\x1b[?65;1;9c"));
        assert!(has_da1_response(b"\x1b[?62c"));
    }

    #[test]
    fn has_da1_response_negative() {
        assert!(!has_da1_response(b""));
        assert!(!has_da1_response(b"\x1b[?c")); // No params
        assert!(!has_da1_response(b"\x1b[65c")); // Missing ?
    }

    // -- TerminalCaps default --

    #[test]
    fn terminal_caps_default_is_no_support() {
        let caps = TerminalCaps::default();
        assert!(!caps.kitty_graphics);
        assert!(!caps.kitty_keyboard);
        assert!(!caps.pixel_mouse);
        assert_eq!(caps.cell_width_px, None);
        assert_eq!(caps.cell_height_px, None);
    }
}
