//! Batched FFI bridge — coarse-grained C ABI surface for `bun:ffi`.
//!
//! Instead of many small FFI calls, JS batches tree mutations into a binary
//! buffer and sends them in a single `apply_mutations()` call.  Rust owns all
//! state (layout tree, text content, event queue, render loop).

use std::collections::HashMap;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use crate::ansi::{self, Color, Style as CellStyle};
use crate::cell::DoubleBuffer;
use crate::focus::{FocusManager, FocusMeta};
use crate::hit_test::HitTester;
use crate::layout::{LayoutNodeId, LayoutTree, NodeStyle};
use crate::terminal_caps;

// ---------------------------------------------------------------------------
// Op codes — must stay in sync with TS `MutationEncoder`
// ---------------------------------------------------------------------------

const OP_CREATE_NODE: u8 = 1;
const OP_REMOVE_NODE: u8 = 2;
const OP_APPEND_CHILD: u8 = 3;
const OP_INSERT_BEFORE: u8 = 4;
const OP_SET_STYLE: u8 = 5;
const OP_SET_TEXT: u8 = 6;
const OP_SET_TEXT_SPANS: u8 = 7;

// ---------------------------------------------------------------------------
// Event types — must stay in sync with TS `EventDecoder`
// ---------------------------------------------------------------------------

const EVENT_KEYBOARD: u8 = 1;
const EVENT_MOUSE: u8 = 2;
const EVENT_RESIZE: u8 = 3;
const EVENT_FOCUS: u8 = 4;
const EVENT_BLUR: u8 = 5;

// ---------------------------------------------------------------------------
// Visual style — bg/fg colors per node (separate from layout style)
// ---------------------------------------------------------------------------

/// How text overflows its container.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum TextOverflow {
    /// Truncate text at the container boundary (default).
    #[default]
    Clip,
    /// Truncate and show an ellipsis at the end.
    Ellipsis,
}

/// Overflow clipping mode for a node.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Overflow {
    #[default]
    Visible,
    Hidden,
}

/// Border preset — determines which Unicode box-drawing characters to use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BorderPreset {
    Single,
    Round,
    Double,
    Bold,
}

impl BorderPreset {
    /// Returns (tl, tr, bl, br, horizontal, vertical) box-drawing characters.
    const fn chars(self) -> (char, char, char, char, char, char) {
        match self {
            Self::Single => (
                '\u{250c}', '\u{2510}', '\u{2514}', '\u{2518}', '\u{2500}', '\u{2502}',
            ),
            Self::Round => (
                '\u{256d}', '\u{256e}', '\u{2570}', '\u{256f}', '\u{2500}', '\u{2502}',
            ),
            Self::Double => (
                '\u{2554}', '\u{2557}', '\u{255a}', '\u{255d}', '\u{2550}', '\u{2551}',
            ),
            Self::Bold => (
                '\u{250f}', '\u{2513}', '\u{2517}', '\u{251b}', '\u{2501}', '\u{2503}',
            ),
        }
    }

    fn from_str(s: &str) -> Option<Self> {
        match s {
            "single" => Some(Self::Single),
            "round" => Some(Self::Round),
            "double" => Some(Self::Double),
            "bold" => Some(Self::Bold),
            _ => None,
        }
    }
}

/// Border style for a node.
#[derive(Clone, Debug)]
struct BorderStyle {
    preset: BorderPreset,
    color: Option<Color>,
}

/// Visual (non-layout) style properties for a node.
#[derive(Clone, Debug, Default)]
#[allow(clippy::struct_excessive_bools)]
struct NodeVisualStyle {
    /// Background color.
    bg: Option<Color>,
    /// Foreground (text) color.
    fg: Option<Color>,
    /// Bold text.
    bold: bool,
    /// Italic text.
    italic: bool,
    /// Text overflow behaviour.
    text_overflow: TextOverflow,
    /// Underline text.
    underline: bool,
    /// Strikethrough text.
    strikethrough: bool,
    /// Dim text.
    dim: bool,
    /// Overflow clipping mode.
    overflow: Overflow,
    /// Border style.
    border: Option<BorderStyle>,
}

// ---------------------------------------------------------------------------
// Text span — per-character color override within text content
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct TextColorSpan {
    start: u16,
    end: u16,
    fg: Color,
}

// ---------------------------------------------------------------------------
// Global engine state
// ---------------------------------------------------------------------------

struct EngineState {
    layout: LayoutTree,
    /// Maps user-facing u32 node ids to Taffy `LayoutNodeId` handles.
    node_map: HashMap<u32, LayoutNodeId>,
    /// Text content per node (`node_id` to string).
    text_content: HashMap<u32, String>,
    /// Per-node text color spans for inline styling.
    text_spans: HashMap<u32, Vec<TextColorSpan>>,
    /// Pending events encoded as binary.
    event_buffer: Vec<u8>,
    /// Root node id (first node created, or explicitly set).
    root_node: Option<u32>,
    /// Whether a render has been requested.
    dirty: bool,
    /// Available columns for layout.
    cols: f32,
    /// Available rows for layout.
    rows: f32,
    /// Event callback function pointer (JS side).
    event_callback: Option<extern "C" fn(*const u8, u32)>,
    /// Focus management.
    focus: FocusManager,
    /// Hit-testing engine.
    hit_tester: HitTester,
    /// Reverse map from layout node ids to user-facing u32 ids.
    reverse_node_map: HashMap<LayoutNodeId, u32>,
    /// Double buffer for efficient terminal rendering.
    double_buf: DoubleBuffer,
    /// Visual (non-layout) style per node.
    visual_styles: HashMap<u32, NodeVisualStyle>,
    /// Output writer — `None` means write to real stdout.
    output: Option<Vec<u8>>,
    /// Detected terminal capabilities.
    terminal_caps: terminal_caps::TerminalCaps,
}

impl EngineState {
    fn new() -> Self {
        Self {
            layout: LayoutTree::new(),
            node_map: HashMap::new(),
            text_content: HashMap::new(),
            text_spans: HashMap::new(),
            event_buffer: Vec::new(),
            root_node: None,
            dirty: false,
            cols: 80.0,
            rows: 24.0,
            event_callback: None,
            focus: FocusManager::new(),
            hit_tester: HitTester::new(),
            reverse_node_map: HashMap::new(),
            double_buf: DoubleBuffer::new(80, 24),
            visual_styles: HashMap::new(),
            output: None,
            terminal_caps: terminal_caps::detect(),
        }
    }

    /// Look up the user-facing u32 id for a layout node id.
    fn user_id(&self, layout_id: LayoutNodeId) -> Option<u32> {
        self.reverse_node_map.get(&layout_id).copied()
    }

    /// Write bytes to the configured output target.
    fn write_output(&mut self, data: &[u8]) {
        if let Some(ref mut buf) = self.output {
            buf.extend_from_slice(data);
        } else {
            let _ = std::io::stdout().lock().write_all(data);
            let _ = std::io::stdout().lock().flush();
        }
    }

    /// Ensure the double buffer matches the current cols/rows dimensions.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn ensure_buffer_size(&mut self) {
        let w = self.cols as usize;
        let h = self.rows as usize;
        if self.double_buf.width() != w || self.double_buf.height() != h {
            self.double_buf.resize(w, h);
        }
    }
}

static ENGINE: Mutex<Option<EngineState>> = Mutex::new(None);
static RENDER_LOOP_RUNNING: AtomicBool = AtomicBool::new(false);

/// Helper: lock the engine and run the closure.
///
/// # Panics
///
/// Panics if the engine has not been initialised via `init()`.
fn with_engine<F, R>(f: F) -> R
where
    F: FnOnce(&mut EngineState) -> R,
{
    let mut guard = ENGINE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // SAFETY: callers must ensure init() was called before any other FFI fn.
    #[allow(clippy::expect_used)]
    let state = guard
        .as_mut()
        .expect("engine not initialised — call init() first");
    f(state)
}

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

/// Capabilities returned by `init()`.
///
/// Exposed as a `#[repr(C)]` struct so `bun:ffi` can read the fields
/// directly without parsing a JSON string.
#[repr(C)]
pub struct InitResult {
    /// Semver major version.
    pub version_major: u16,
    /// Semver minor version.
    pub version_minor: u16,
    /// Semver patch version.
    pub version_patch: u16,
    /// Non-zero if the batched FFI protocol is supported.
    pub batched_ffi: u8,
}

/// Initialise the engine.  Writes capabilities into `out_ptr`.
///
/// Enters the alternate screen and hides the cursor for clean terminal
/// rendering.
///
/// # Safety
///
/// - Must be called exactly once before any other FFI function.
/// - `out_ptr` must point to a writable `InitResult`.
#[no_mangle]
pub unsafe extern "C" fn init(out_ptr: *mut InitResult) {
    // Enter alternate screen and hide cursor.
    let _ = crate::screen::enter();
    let _ = std::io::stdout().lock().write_all(&ansi::cursor_hide());
    let _ = std::io::stdout().lock().flush();

    let mut guard = ENGINE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *guard = Some(EngineState::new());
    if !out_ptr.is_null() {
        unsafe {
            out_ptr.write(InitResult {
                version_major: 0,
                version_minor: 1,
                version_patch: 0,
                batched_ffi: 1,
            });
        }
    }
}

/// Shut down the engine and release all resources.
///
/// Shows the cursor and exits the alternate screen.
#[no_mangle]
pub extern "C" fn shutdown() {
    RENDER_LOOP_RUNNING.store(false, Ordering::SeqCst);
    let mut guard = ENGINE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *guard = None;

    // Show cursor and exit alternate screen.
    let _ = std::io::stdout().lock().write_all(&ansi::cursor_show());
    let _ = std::io::stdout().lock().flush();
    let _ = crate::screen::exit();
}

/// Return terminal capabilities as a JSON string.
///
/// Writes up to `max_len` bytes of JSON into `out_ptr` and returns the
/// number of bytes written.  If the buffer is too small the output is
/// truncated (caller should allocate generously, e.g. 1024 bytes).
///
/// # Safety
///
/// - `out_ptr` must point to a writable buffer of at least `max_len` bytes.
#[no_mangle]
#[allow(clippy::cast_possible_truncation)]
pub unsafe extern "C" fn get_terminal_caps(out_ptr: *mut u8, max_len: u32) -> u32 {
    with_engine(|state| {
        let json = serde_json::to_string(&state.terminal_caps).unwrap_or_default();
        let bytes = json.as_bytes();
        let n = bytes.len().min(max_len as usize);
        if !out_ptr.is_null() && n > 0 {
            unsafe {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), out_ptr, n);
            }
        }
        n as u32
    })
}

/// Set the viewport (available terminal) size.  The next `render_frame()`
/// call will use these dimensions for layout computation and will resize
/// the internal cell buffers accordingly.
#[no_mangle]
pub extern "C" fn set_viewport_size(cols: u16, rows: u16) {
    with_engine(|state| {
        state.cols = f32::from(cols);
        state.rows = f32::from(rows);
    });
}

// ---------------------------------------------------------------------------
// Test-mode lifecycle (no terminal side effects)
// ---------------------------------------------------------------------------

/// Initialise the engine in test mode.  Like [`init`] but:
/// - Captures output into an internal buffer instead of writing to stdout.
/// - Uses the provided `cols`/`rows` instead of the real terminal size.
/// - Does **not** enter the alternate screen or hide the cursor.
///
/// # Safety
///
/// - Must be called exactly once before any other FFI function.
/// - `out_ptr` must point to a writable `InitResult`.
#[no_mangle]
pub unsafe extern "C" fn init_test_mode(cols: u16, rows: u16, out_ptr: *mut InitResult) {
    let mut guard = ENGINE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut state = EngineState::new();
    state.output = Some(Vec::new());
    state.cols = f32::from(cols);
    state.rows = f32::from(rows);
    state.double_buf = DoubleBuffer::new(usize::from(cols), usize::from(rows));
    *guard = Some(state);
    if !out_ptr.is_null() {
        unsafe {
            out_ptr.write(InitResult {
                version_major: 0,
                version_minor: 1,
                version_patch: 0,
                batched_ffi: 1,
            });
        }
    }
}

/// Copy the captured output bytes into `out_ptr` (up to `max_len`), clear the
/// internal buffer, and return the number of bytes written.
///
/// # Safety
///
/// - `out_ptr` must point to a writable byte array of at least `max_len` bytes.
#[no_mangle]
pub unsafe extern "C" fn get_rendered_output(out_ptr: *mut u8, max_len: u32) -> u32 {
    if out_ptr.is_null() || max_len == 0 {
        return 0;
    }
    with_engine(|state| {
        let Some(ref mut buf) = state.output else {
            return 0;
        };
        #[allow(clippy::cast_possible_truncation)]
        let copy_len = buf.len().min(max_len as usize);
        if copy_len > 0 {
            unsafe {
                std::ptr::copy_nonoverlapping(buf.as_ptr(), out_ptr, copy_len);
            }
        }
        buf.clear();
        #[allow(clippy::cast_possible_truncation)]
        let result = copy_len as u32;
        result
    })
}

/// Shut down the engine in test mode — drops all state but does **not**
/// write cursor-show or exit the alternate screen.
#[no_mangle]
pub extern "C" fn shutdown_test_mode() {
    RENDER_LOOP_RUNNING.store(false, Ordering::SeqCst);
    let mut guard = ENGINE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *guard = None;
}

// ---------------------------------------------------------------------------
// Mutation decoding
// ---------------------------------------------------------------------------

/// A zero-copy reader over the mutation buffer.
struct MutationReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> MutationReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    fn read_u8(&mut self) -> Option<u8> {
        if self.pos < self.data.len() {
            let v = self.data[self.pos];
            self.pos += 1;
            Some(v)
        } else {
            None
        }
    }

    fn read_u16(&mut self) -> Option<u16> {
        if self.pos + 2 <= self.data.len() {
            let v = u16::from_le_bytes([self.data[self.pos], self.data[self.pos + 1]]);
            self.pos += 2;
            Some(v)
        } else {
            None
        }
    }

    fn read_u32(&mut self) -> Option<u32> {
        if self.pos + 4 <= self.data.len() {
            let v = u32::from_le_bytes([
                self.data[self.pos],
                self.data[self.pos + 1],
                self.data[self.pos + 2],
                self.data[self.pos + 3],
            ]);
            self.pos += 4;
            Some(v)
        } else {
            None
        }
    }

    fn read_bytes(&mut self, len: usize) -> Option<&'a [u8]> {
        if self.pos + len <= self.data.len() {
            let slice = &self.data[self.pos..self.pos + len];
            self.pos += len;
            Some(slice)
        } else {
            None
        }
    }
}

/// Parse a JSON style blob into a `NodeStyle`.
///
/// Accepts a minimal subset:
/// ```json
/// { "width": 10, "height": 5, "flexDirection": "column", "flexGrow": 1 }
/// ```
///
/// Unrecognised keys are silently ignored so the format can grow.
#[allow(clippy::too_many_lines)]
fn parse_style_json(json: &[u8]) -> NodeStyle {
    let s = std::str::from_utf8(json).unwrap_or("{}");
    let mut style = NodeStyle::default();

    if let Some(v) = json_extract_f32(s, "width") {
        style.width = crate::layout::Dim::Cells(v);
    }
    if let Some(v) = json_extract_f32(s, "height") {
        style.height = crate::layout::Dim::Cells(v);
    }
    if let Some(v) = json_extract_f32(s, "minWidth") {
        style.min_width = crate::layout::Dim::Cells(v);
    }
    if let Some(v) = json_extract_f32(s, "minHeight") {
        style.min_height = crate::layout::Dim::Cells(v);
    }
    if let Some(v) = json_extract_f32(s, "maxWidth") {
        style.max_width = crate::layout::Dim::Cells(v);
    }
    if let Some(v) = json_extract_f32(s, "maxHeight") {
        style.max_height = crate::layout::Dim::Cells(v);
    }
    if let Some(v) = json_extract_f32(s, "flexGrow") {
        if let crate::layout::DisplayMode::Flex(ref mut flex) = style.display {
            flex.grow = v;
        }
    }
    if let Some(v) = json_extract_f32(s, "flexShrink") {
        if let crate::layout::DisplayMode::Flex(ref mut flex) = style.display {
            flex.shrink = v;
        }
    }
    if let Some(dir) = json_extract_str(s, "flexDirection") {
        if let crate::layout::DisplayMode::Flex(ref mut flex) = style.display {
            flex.direction = match dir {
                "column" => crate::layout::FlexDir::Column,
                "row-reverse" => crate::layout::FlexDir::RowReverse,
                "column-reverse" => crate::layout::FlexDir::ColumnReverse,
                _ => crate::layout::FlexDir::Row,
            };
        }
    }
    if let Some(v) = json_extract_f32(s, "flexBasis") {
        if let crate::layout::DisplayMode::Flex(ref mut flex) = style.display {
            flex.basis = crate::layout::Dim::Cells(v);
        }
    }
    if let Some(jc) = json_extract_str(s, "justifyContent") {
        if let crate::layout::DisplayMode::Flex(ref mut flex) = style.display {
            flex.justify = match jc {
                "end" => crate::layout::JustifyContent::End,
                "center" => crate::layout::JustifyContent::Center,
                "space-between" => crate::layout::JustifyContent::SpaceBetween,
                "space-around" => crate::layout::JustifyContent::SpaceAround,
                "space-evenly" => crate::layout::JustifyContent::SpaceEvenly,
                _ => crate::layout::JustifyContent::Start,
            };
        }
    }
    if let Some(ai) = json_extract_str(s, "alignItems") {
        if let crate::layout::DisplayMode::Flex(ref mut flex) = style.display {
            flex.align_items = match ai {
                "end" => crate::layout::AlignItems::End,
                "center" => crate::layout::AlignItems::Center,
                "baseline" => crate::layout::AlignItems::Baseline,
                "start" => crate::layout::AlignItems::Start,
                _ => crate::layout::AlignItems::Stretch,
            };
        }
    }
    // Padding: try as array first, then as scalar.
    if let Some(arr) = json_extract_f32_array(s, "padding") {
        for (i, v) in arr.iter().enumerate().take(4) {
            style.padding[i] = crate::layout::Dim::Cells(*v);
        }
    } else if let Some(v) = json_extract_f32(s, "padding") {
        style.padding = [crate::layout::Dim::Cells(v); 4];
    }
    // Per-side padding overrides.
    if let Some(v) = json_extract_f32(s, "paddingTop") {
        style.padding[0] = crate::layout::Dim::Cells(v);
    }
    if let Some(v) = json_extract_f32(s, "paddingRight") {
        style.padding[1] = crate::layout::Dim::Cells(v);
    }
    if let Some(v) = json_extract_f32(s, "paddingBottom") {
        style.padding[2] = crate::layout::Dim::Cells(v);
    }
    if let Some(v) = json_extract_f32(s, "paddingLeft") {
        style.padding[3] = crate::layout::Dim::Cells(v);
    }
    // Margin: try as array first, then as scalar.
    if let Some(arr) = json_extract_f32_array(s, "margin") {
        for (i, v) in arr.iter().enumerate().take(4) {
            style.margin[i] = crate::layout::Dim::Cells(*v);
        }
    } else if let Some(v) = json_extract_f32(s, "margin") {
        style.margin = [crate::layout::Dim::Cells(v); 4];
    }
    // Per-side margin overrides.
    if let Some(v) = json_extract_f32(s, "marginTop") {
        style.margin[0] = crate::layout::Dim::Cells(v);
    }
    if let Some(v) = json_extract_f32(s, "marginRight") {
        style.margin[1] = crate::layout::Dim::Cells(v);
    }
    if let Some(v) = json_extract_f32(s, "marginBottom") {
        style.margin[2] = crate::layout::Dim::Cells(v);
    }
    if let Some(v) = json_extract_f32(s, "marginLeft") {
        style.margin[3] = crate::layout::Dim::Cells(v);
    }
    // Gap: try as array first, then as scalar.
    if let Some(arr) = json_extract_f32_array(s, "gap") {
        if arr.len() >= 2 {
            style.gap = [
                crate::layout::Dim::Cells(arr[0]),
                crate::layout::Dim::Cells(arr[1]),
            ];
        }
    } else if let Some(v) = json_extract_f32(s, "gap") {
        style.gap = [crate::layout::Dim::Cells(v); 2];
    }

    style
}

/// Parse a hex color string (#RGB or #RRGGBB) into a `Color`.
fn parse_hex_color(hex: &str) -> Option<Color> {
    let hex = hex.strip_prefix('#')?;
    match hex.len() {
        3 => {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()?;
            // Expand 4-bit to 8-bit: 0xA -> 0xAA
            Some(Color::Rgb(r << 4 | r, g << 4 | g, b << 4 | b))
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(Color::Rgb(r, g, b))
        }
        _ => None,
    }
}

/// Parse a JSON style blob and also extract visual style (bg/fg colors, bold, italic).
fn parse_visual_style_json(json: &[u8]) -> NodeVisualStyle {
    let s = std::str::from_utf8(json).unwrap_or("{}");
    let mut vs = NodeVisualStyle::default();
    if let Some(hex) = json_extract_str(s, "backgroundColor") {
        vs.bg = parse_hex_color(hex);
    }
    if let Some(hex) = json_extract_str(s, "color") {
        vs.fg = parse_hex_color(hex);
    }
    if let Some(b) = json_extract_bool(s, "bold") {
        vs.bold = b;
    }
    if let Some(i) = json_extract_bool(s, "italic") {
        vs.italic = i;
    }
    if let Some(to) = json_extract_str(s, "textOverflow") {
        vs.text_overflow = match to {
            "ellipsis" => TextOverflow::Ellipsis,
            _ => TextOverflow::Clip,
        };
    }
    if let Some(u) = json_extract_bool(s, "underline") {
        vs.underline = u;
    }
    if let Some(st) = json_extract_bool(s, "strikethrough") {
        vs.strikethrough = st;
    }
    if let Some(d) = json_extract_bool(s, "dim") {
        vs.dim = d;
    }
    // CSS-like textDecoration: "underline" or "line-through"
    if let Some(td) = json_extract_str(s, "textDecoration") {
        match td {
            "underline" => vs.underline = true,
            "line-through" | "strikethrough" => vs.strikethrough = true,
            _ => {}
        }
    }
    if let Some(ov) = json_extract_str(s, "overflow") {
        vs.overflow = match ov {
            "hidden" => Overflow::Hidden,
            _ => Overflow::Visible,
        };
    }
    if let Some(border_str) = json_extract_str(s, "border") {
        if let Some(preset) = BorderPreset::from_str(border_str) {
            let color = json_extract_str(s, "borderColor").and_then(parse_hex_color);
            vs.border = Some(BorderStyle { preset, color });
        }
    }
    vs
}

/// Extract a float value for a given key from a JSON string (best-effort).
fn json_extract_f32(s: &str, key: &str) -> Option<f32> {
    let pattern = format!("\"{key}\"");
    let idx = s.find(&pattern)?;
    let after_key = &s[idx + pattern.len()..];
    let after_colon = after_key.trim_start().strip_prefix(':')?;
    let after_colon = after_colon.trim_start();
    let end = after_colon
        .find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
        .unwrap_or(after_colon.len());
    after_colon[..end].parse().ok()
}

/// Extract an array of floats for a given key from a JSON string (best-effort).
/// Handles JSON like `"key":[1,2,3,4]`.
fn json_extract_f32_array(s: &str, key: &str) -> Option<Vec<f32>> {
    let pattern = format!("\"{key}\"");
    let idx = s.find(&pattern)?;
    let after_key = &s[idx + pattern.len()..];
    let after_colon = after_key.trim_start().strip_prefix(':')?.trim_start();
    let after_bracket = after_colon.strip_prefix('[')?;
    let end = after_bracket.find(']')?;
    let inner = &after_bracket[..end];
    let values: Vec<f32> = inner
        .split(',')
        .filter_map(|v| v.trim().parse::<f32>().ok())
        .collect();
    if values.is_empty() {
        None
    } else {
        Some(values)
    }
}

/// Extract a boolean value for a given key from a JSON string (best-effort).
fn json_extract_bool(s: &str, key: &str) -> Option<bool> {
    let pattern = format!("\"{key}\"");
    let idx = s.find(&pattern)?;
    let after_key = &s[idx + pattern.len()..];
    let after_colon = after_key.trim_start().strip_prefix(':')?.trim_start();
    if after_colon.starts_with("true") {
        Some(true)
    } else if after_colon.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

/// Extract a string value for a given key from a JSON string (best-effort).
fn json_extract_str<'a>(s: &'a str, key: &str) -> Option<&'a str> {
    let pattern = format!("\"{key}\"");
    let idx = s.find(&pattern)?;
    let after_key = &s[idx + pattern.len()..];
    let after_colon = after_key.trim_start().strip_prefix(':')?.trim_start();
    let after_quote = after_colon.strip_prefix('"')?;
    let end = after_quote.find('"')?;
    Some(&after_quote[..end])
}

/// Process a single mutation op from the reader, returning `true` to continue.
fn process_mutation(reader: &mut MutationReader<'_>, state: &mut EngineState) -> bool {
    let Some(op) = reader.read_u8() else {
        return false;
    };
    let ok = match op {
        OP_CREATE_NODE => mut_create_node(reader, state),
        OP_REMOVE_NODE => mut_remove_node(reader, state),
        OP_APPEND_CHILD => mut_append_child(reader, state),
        OP_INSERT_BEFORE => mut_insert_before(reader, state),
        OP_SET_STYLE => mut_set_style(reader, state),
        OP_SET_TEXT => mut_set_text(reader, state),
        OP_SET_TEXT_SPANS => mut_set_text_spans(reader, state),
        _ => false,
    };
    if ok {
        state.dirty = true;
    }
    ok
}

fn mut_create_node(reader: &mut MutationReader<'_>, state: &mut EngineState) -> bool {
    let Some(node_id) = reader.read_u32() else {
        return false;
    };
    let Some(json_len) = reader.read_u16() else {
        return false;
    };
    let Some(json_bytes) = reader.read_bytes(json_len as usize) else {
        return false;
    };
    let style = parse_style_json(json_bytes);
    let vs = parse_visual_style_json(json_bytes);
    if let Ok(layout_id) = state.layout.add_leaf(&style) {
        state.node_map.insert(node_id, layout_id);
        state.reverse_node_map.insert(layout_id, node_id);
        state.visual_styles.insert(node_id, vs);
        if state.root_node.is_none() {
            state.root_node = Some(node_id);
        }
    }
    true
}

fn mut_remove_node(reader: &mut MutationReader<'_>, state: &mut EngineState) -> bool {
    let Some(node_id) = reader.read_u32() else {
        return false;
    };
    if let Some(layout_id) = state.node_map.remove(&node_id) {
        state.reverse_node_map.remove(&layout_id);
        state.focus.remove_meta(layout_id);
        state.hit_tester.remove_meta(layout_id);
        let _ = state.layout.remove(layout_id);
    }
    state.text_content.remove(&node_id);
    state.text_spans.remove(&node_id);
    state.visual_styles.remove(&node_id);
    true
}

fn mut_append_child(reader: &mut MutationReader<'_>, state: &mut EngineState) -> bool {
    let Some(parent_id) = reader.read_u32() else {
        return false;
    };
    let Some(child_id) = reader.read_u32() else {
        return false;
    };
    if let (Some(&p), Some(&c)) = (
        state.node_map.get(&parent_id),
        state.node_map.get(&child_id),
    ) {
        let _ = state.layout.add_child(p, c);
    }
    true
}

fn mut_insert_before(reader: &mut MutationReader<'_>, state: &mut EngineState) -> bool {
    let Some(parent_id) = reader.read_u32() else {
        return false;
    };
    let Some(child_id) = reader.read_u32() else {
        return false;
    };
    let Some(_before_id) = reader.read_u32() else {
        return false;
    };
    // Taffy doesn't have insert_before — append for now.
    if let (Some(&p), Some(&c)) = (
        state.node_map.get(&parent_id),
        state.node_map.get(&child_id),
    ) {
        let _ = state.layout.add_child(p, c);
    }
    true
}

fn mut_set_style(reader: &mut MutationReader<'_>, state: &mut EngineState) -> bool {
    let Some(node_id) = reader.read_u32() else {
        return false;
    };
    let Some(json_len) = reader.read_u16() else {
        return false;
    };
    let Some(json_bytes) = reader.read_bytes(json_len as usize) else {
        return false;
    };
    let style = parse_style_json(json_bytes);
    let vs = parse_visual_style_json(json_bytes);
    if let Some(&layout_id) = state.node_map.get(&node_id) {
        let _ = state.layout.set_style(layout_id, &style);
    }
    state.visual_styles.insert(node_id, vs);
    true
}

fn mut_set_text(reader: &mut MutationReader<'_>, state: &mut EngineState) -> bool {
    let Some(node_id) = reader.read_u32() else {
        return false;
    };
    let Some(text_len) = reader.read_u16() else {
        return false;
    };
    let Some(text_bytes) = reader.read_bytes(text_len as usize) else {
        return false;
    };
    if let Ok(text) = std::str::from_utf8(text_bytes) {
        state.text_content.insert(node_id, text.to_owned());
    }
    true
}

fn mut_set_text_spans(reader: &mut MutationReader<'_>, state: &mut EngineState) -> bool {
    let Some(node_id) = reader.read_u32() else {
        return false;
    };
    let Some(span_count) = reader.read_u16() else {
        return false;
    };
    let mut spans = Vec::with_capacity(span_count as usize);
    for _ in 0..span_count {
        let Some(start) = reader.read_u16() else {
            return false;
        };
        let Some(end) = reader.read_u16() else {
            return false;
        };
        let Some(r) = reader.read_u8() else {
            return false;
        };
        let Some(g) = reader.read_u8() else {
            return false;
        };
        let Some(b) = reader.read_u8() else {
            return false;
        };
        spans.push(TextColorSpan {
            start,
            end,
            fg: Color::Rgb(r, g, b),
        });
    }
    if spans.is_empty() {
        state.text_spans.remove(&node_id);
    } else {
        state.text_spans.insert(node_id, spans);
    }
    true
}

/// Apply a batch of binary-encoded mutations.
///
/// # Safety
///
/// `buffer_ptr` must point to a valid byte array of at least `buffer_len` bytes.
#[no_mangle]
pub unsafe extern "C" fn apply_mutations(buffer_ptr: *const u8, buffer_len: u32) {
    if buffer_ptr.is_null() || buffer_len == 0 {
        return;
    }
    let data = unsafe { std::slice::from_raw_parts(buffer_ptr, buffer_len as usize) };
    let mut reader = MutationReader::new(data);

    with_engine(|state| {
        while reader.remaining() > 0 {
            if !process_mutation(&mut reader, state) {
                break;
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

/// Check whether a cell at (row, col) is inside the active clip rectangle.
#[allow(clippy::cast_precision_loss)]
fn in_clip(row: usize, col: usize, clip: Option<(f32, f32, f32, f32)>) -> bool {
    if let Some((cx, cy, cw, ch)) = clip {
        let col_f = col as f32;
        let row_f = row as f32;
        col_f >= cx && col_f < cx + cw && row_f >= cy && row_f < cy + ch
    } else {
        true
    }
}

/// Walk the layout tree recursively and paint each node into the back buffer.
///
/// `parent_x` / `parent_y` are the absolute position of the parent so that
/// each child's relative layout coordinates can be converted to absolute
/// positions in the cell buffer.
///
/// `inherited_bg` / `inherited_fg` are the resolved colors from ancestor
/// nodes, allowing children to inherit colours they don't override.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::similar_names
)]
#[allow(clippy::fn_params_excessive_bools)]
fn paint_node(
    state: &EngineState,
    back: &mut crate::cell::CellBuffer,
    node_id: u32,
    parent_x: f32,
    parent_y: f32,
    inherited_bg: Option<Color>,
    inherited_fg: Option<Color>,
    inherited_bold: bool,
    inherited_italic: bool,
    inherited_underline: bool,
    inherited_strikethrough: bool,
    inherited_dim: bool,
    clip_rect: Option<(f32, f32, f32, f32)>,
) {
    let Some(&layout_id) = state.node_map.get(&node_id) else {
        return;
    };
    let Ok(cl) = state.layout.get_layout(layout_id) else {
        return;
    };

    let abs_x = parent_x + cl.x;
    let abs_y = parent_y + cl.y;
    let x0 = abs_x as usize;
    let y0 = abs_y as usize;
    let w = cl.width as usize;
    let h = cl.height as usize;

    // Build cell style from visual style, inheriting from ancestors.
    let vs = state.visual_styles.get(&node_id);
    let own_bg = vs.and_then(|v| v.bg);
    let own_fg = vs.and_then(|v| v.fg);
    let resolved_bg = own_bg.or(inherited_bg);
    let resolved_fg = own_fg.or(inherited_fg);
    let resolved_bold = vs.map_or(
        inherited_bold,
        |v| if v.bold { true } else { inherited_bold },
    );
    let resolved_italic = vs.map_or(inherited_italic, |v| {
        if v.italic {
            true
        } else {
            inherited_italic
        }
    });
    let resolved_underline = vs.map_or(inherited_underline, |v| {
        if v.underline {
            true
        } else {
            inherited_underline
        }
    });
    let resolved_strikethrough = vs.map_or(inherited_strikethrough, |v| {
        if v.strikethrough {
            true
        } else {
            inherited_strikethrough
        }
    });
    let resolved_dim = vs.map_or(inherited_dim, |v| if v.dim { true } else { inherited_dim });

    // Paint background only if we have an explicit bg color (own or inherited).
    if resolved_bg.is_some() {
        let cell_style = CellStyle {
            bg: resolved_bg,
            fg: resolved_fg,
            bold: resolved_bold,
            italic: resolved_italic,
            underline: resolved_underline,
            strikethrough: resolved_strikethrough,
            dim: resolved_dim,
            ..CellStyle::new()
        };
        for row in y0..y0 + h {
            for col in x0..x0 + w {
                if !in_clip(row, col, clip_rect) {
                    continue;
                }
                if let Some(cell) = back.get_mut(row, col) {
                    cell.ch = ' ';
                    cell.style = cell_style;
                }
            }
        }
    }

    // Paint border if present.
    if let Some(border) = vs.and_then(|v| v.border.as_ref()) {
        if w >= 2 && h >= 2 {
            let (tl, tr, bl, br, horiz, vert) = border.preset.chars();
            let border_style = CellStyle {
                fg: border.color.or(resolved_fg),
                bg: inherited_bg,
                ..CellStyle::new()
            };

            // Corners
            if let Some(cell) = back.get_mut(y0, x0) {
                cell.ch = tl;
                cell.style = border_style;
            }
            if let Some(cell) = back.get_mut(y0, x0 + w - 1) {
                cell.ch = tr;
                cell.style = border_style;
            }
            if let Some(cell) = back.get_mut(y0 + h - 1, x0) {
                cell.ch = bl;
                cell.style = border_style;
            }
            if let Some(cell) = back.get_mut(y0 + h - 1, x0 + w - 1) {
                cell.ch = br;
                cell.style = border_style;
            }

            // Horizontal edges (top and bottom)
            for col in (x0 + 1)..(x0 + w - 1) {
                if let Some(cell) = back.get_mut(y0, col) {
                    cell.ch = horiz;
                    cell.style = border_style;
                }
                if let Some(cell) = back.get_mut(y0 + h - 1, col) {
                    cell.ch = horiz;
                    cell.style = border_style;
                }
            }

            // Vertical edges (left and right)
            for row in (y0 + 1)..(y0 + h - 1) {
                if let Some(cell) = back.get_mut(row, x0) {
                    cell.ch = vert;
                    cell.style = border_style;
                }
                if let Some(cell) = back.get_mut(row, x0 + w - 1) {
                    cell.ch = vert;
                    cell.style = border_style;
                }
            }
        }
    }

    // Paint text content — use resolved colours, text attributes, and per-span overrides.
    // Respect text_overflow: Clip stops at boundary, Ellipsis places an ellipsis at the end.
    let text_overflow = vs.map_or(TextOverflow::Clip, |v| v.text_overflow);
    if let Some(text) = state.text_content.get(&node_id) {
        if !text.is_empty() && w > 0 {
            let char_count = text.chars().count();
            let needs_ellipsis = text_overflow == TextOverflow::Ellipsis && char_count > w;
            let paint_limit = if needs_ellipsis { w - 1 } else { w };
            let spans = state.text_spans.get(&node_id);
            for (i, ch) in text.chars().enumerate() {
                if i >= paint_limit {
                    break;
                }
                let c = x0 + i;
                if c >= back.width() {
                    break;
                }
                if y0 < back.height() {
                    if !in_clip(y0, c, clip_rect) {
                        continue;
                    }
                    if let Some(cell) = back.get_mut(y0, c) {
                        cell.ch = ch;
                        cell.style.bold = resolved_bold;
                        cell.style.italic = resolved_italic;
                        cell.style.underline = resolved_underline;
                        cell.style.strikethrough = resolved_strikethrough;
                        cell.style.dim = resolved_dim;
                        let span_fg = spans.and_then(|ss| {
                            let idx = i as u16;
                            ss.iter()
                                .rev()
                                .find(|s| idx >= s.start && idx < s.end)
                                .map(|s| s.fg)
                        });
                        cell.style.fg = span_fg.or(resolved_fg);
                        if let Some(bg) = resolved_bg {
                            cell.style.bg = Some(bg);
                        }
                    }
                }
            }
            // Place ellipsis character at the last position.
            if needs_ellipsis {
                let c = x0 + paint_limit;
                if c < back.width() && y0 < back.height() {
                    if let Some(cell) = back.get_mut(y0, c) {
                        cell.ch = '\u{2026}'; // ellipsis
                        cell.style.bold = resolved_bold;
                        cell.style.italic = resolved_italic;
                        if let Some(fg) = resolved_fg {
                            cell.style.fg = Some(fg);
                        }
                        if let Some(bg) = resolved_bg {
                            cell.style.bg = Some(bg);
                        }
                    }
                }
            }
        }
    }

    // Determine the clip rect for children.
    let own_overflow = vs.map_or(Overflow::Visible, |v| v.overflow);
    let child_clip = if own_overflow == Overflow::Hidden {
        let own_rect = (abs_x, abs_y, cl.width, cl.height);
        Some(intersect_clip(clip_rect, own_rect))
    } else {
        clip_rect
    };

    // Recurse into children.
    if let Ok(children) = state.layout.children(layout_id) {
        for child_lid in children {
            if let Some(&child_uid) = state.reverse_node_map.get(&child_lid) {
                paint_node(
                    state,
                    back,
                    child_uid,
                    abs_x,
                    abs_y,
                    resolved_bg,
                    resolved_fg,
                    resolved_bold,
                    resolved_italic,
                    resolved_underline,
                    resolved_strikethrough,
                    resolved_dim,
                    child_clip,
                );
            }
        }
    }
}

/// Intersect an optional inherited clip rect with a new rect.
fn intersect_clip(
    inherited: Option<(f32, f32, f32, f32)>,
    rect: (f32, f32, f32, f32),
) -> (f32, f32, f32, f32) {
    if let Some((ix, iy, iw, ih)) = inherited {
        let x0 = ix.max(rect.0);
        let y0 = iy.max(rect.1);
        let x1 = (ix + iw).min(rect.0 + rect.2);
        let y1 = (iy + ih).min(rect.1 + rect.3);
        (x0, y0, (x1 - x0).max(0.0), (y1 - y0).max(0.0))
    } else {
        rect
    }
}

/// Run the full render pipeline: layout, paint, diff, output, flush events.
#[no_mangle]
pub extern "C" fn render_frame() {
    with_engine(|state| {
        // 1. Compute layout and rebuild hit-test grid.
        if let Some(root_id) = state.root_node {
            if let Some(&layout_id) = state.node_map.get(&root_id) {
                let _ = state.layout.compute(layout_id, state.cols, state.rows);

                // Rebuild hit-test grid after layout so hit_test queries are up-to-date.
                state.hit_tester.set_root(layout_id);
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let cols = state.cols as usize;
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let rows = state.rows as usize;
                let _ = state.hit_tester.build_grid(&state.layout, cols, rows);
            }
        }

        // 2. Paint cells and diff only when dirty.
        if state.dirty {
            state.ensure_buffer_size();

            // Clear back buffer.
            state.double_buf.back_mut().clear();

            // Walk all nodes and paint them.
            if let Some(root_id) = state.root_node {
                // We need to pass state immutably while mutating the buffer.
                // Temporarily take the back buffer out so we can pass state
                // to `paint_node` while mutating the buffer.
                let mut back_buf = {
                    let back = state.double_buf.back_mut();
                    let w = back.width();
                    let h = back.height();
                    let mut tmp = crate::cell::CellBuffer::new(w, h);
                    std::mem::swap(back, &mut tmp);
                    tmp
                };

                paint_node(
                    state,
                    &mut back_buf,
                    root_id,
                    0.0,
                    0.0,
                    None,
                    None,
                    false,
                    false,
                    false,
                    false,
                    false,
                    None,
                );

                // Swap the painted buffer back.
                std::mem::swap(state.double_buf.back_mut(), &mut back_buf);
            }

            // Output: use full_render for test mode (captured output),
            // diff for real terminal rendering.
            let is_test_mode = state.output.is_some();
            if is_test_mode {
                let rendered = state.double_buf.full_render();
                if !rendered.is_empty() {
                    state.write_output(&rendered);
                }
            } else {
                let diff = state.double_buf.diff();
                if !diff.is_empty() {
                    state.write_output(&diff);
                }
            }

            // Swap buffers.
            state.double_buf.swap_no_clear();
        }

        state.dirty = false;

        // 3. Flush events to JS.
        if !state.event_buffer.is_empty() {
            if let Some(cb) = state.event_callback {
                let ptr = state.event_buffer.as_ptr();
                #[allow(clippy::cast_possible_truncation)]
                let len = state.event_buffer.len() as u32;
                cb(ptr, len);
            }
            state.event_buffer.clear();
        }
    });
}

/// Mark the scene as dirty so the next frame tick re-renders.
#[no_mangle]
pub extern "C" fn request_render() {
    with_engine(|state| {
        state.dirty = true;
    });
}

/// Start a render loop capped at `fps` frames per second.
#[no_mangle]
pub extern "C" fn start_render_loop(fps: u32) {
    if RENDER_LOOP_RUNNING.load(Ordering::SeqCst) {
        return;
    }
    RENDER_LOOP_RUNNING.store(true, Ordering::SeqCst);

    let frame_duration = std::time::Duration::from_micros(if fps == 0 {
        16_667
    } else {
        1_000_000 / u64::from(fps)
    });

    std::thread::spawn(move || {
        while RENDER_LOOP_RUNNING.load(Ordering::SeqCst) {
            let needs_render = with_engine(|state| state.dirty);
            if needs_render {
                render_frame();
            }
            std::thread::sleep(frame_duration);
        }
    });
}

/// Stop the render loop.
#[no_mangle]
pub extern "C" fn stop_render_loop() {
    RENDER_LOOP_RUNNING.store(false, Ordering::SeqCst);
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/// Register a callback that Rust invokes with batched events.
///
/// # Safety
///
/// `fn_ptr` must be a valid function pointer for the lifetime of the engine.
#[no_mangle]
pub unsafe extern "C" fn register_event_callback(fn_ptr: extern "C" fn(*const u8, u32)) {
    with_engine(|state| {
        state.event_callback = Some(fn_ptr);
    });
}

/// Push a keyboard event into the event buffer.
#[allow(dead_code)]
fn push_keyboard_event(state: &mut EngineState, key_code: u32, modifiers: u8, event_type: u8) {
    state.event_buffer.push(EVENT_KEYBOARD);
    state
        .event_buffer
        .extend_from_slice(&key_code.to_le_bytes());
    state.event_buffer.push(modifiers);
    state.event_buffer.push(event_type);
}

/// Push a mouse event into the event buffer.
#[allow(dead_code, clippy::too_many_arguments)]
fn push_mouse_event(
    state: &mut EngineState,
    button: u8,
    x: u16,
    y: u16,
    pixel_x: u16,
    pixel_y: u16,
    modifiers: u8,
    node_id: u32,
) {
    state.event_buffer.push(EVENT_MOUSE);
    state.event_buffer.push(button);
    state.event_buffer.extend_from_slice(&x.to_le_bytes());
    state.event_buffer.extend_from_slice(&y.to_le_bytes());
    state.event_buffer.extend_from_slice(&pixel_x.to_le_bytes());
    state.event_buffer.extend_from_slice(&pixel_y.to_le_bytes());
    state.event_buffer.push(modifiers);
    state.event_buffer.extend_from_slice(&node_id.to_le_bytes());
}

/// Push a resize event into the event buffer.
#[allow(dead_code)]
fn push_resize_event(
    state: &mut EngineState,
    cols: u16,
    rows: u16,
    pixel_width: u16,
    pixel_height: u16,
) {
    state.event_buffer.push(EVENT_RESIZE);
    state.event_buffer.extend_from_slice(&cols.to_le_bytes());
    state.event_buffer.extend_from_slice(&rows.to_le_bytes());
    state
        .event_buffer
        .extend_from_slice(&pixel_width.to_le_bytes());
    state
        .event_buffer
        .extend_from_slice(&pixel_height.to_le_bytes());
}

// ---------------------------------------------------------------------------
// Input system — keyboard / mouse callbacks, focus management
// ---------------------------------------------------------------------------

/// Push a keyboard event from raw terminal input.
#[no_mangle]
pub extern "C" fn push_key_event(key_code: u32, modifiers: u8, event_type: u8) {
    with_engine(|state| {
        push_keyboard_event(state, key_code, modifiers, event_type);
    });
}

/// Push a mouse event with automatic hit-testing at `(x, y)`.
#[no_mangle]
pub extern "C" fn push_mouse_event_with_hit_test(
    button: u8,
    x: u16,
    y: u16,
    pixel_x: u16,
    pixel_y: u16,
    modifiers: u8,
) {
    with_engine(|state| {
        let fx = f32::from(x);
        let fy = f32::from(y);

        let node_id = if let Some(root_id) = state.root_node {
            if let Some(&root_layout_id) = state.node_map.get(&root_id) {
                state.hit_tester.set_root(root_layout_id);
                state
                    .hit_tester
                    .hit_test(&state.layout, fx, fy)
                    .ok()
                    .and_then(|r| r.target)
                    .and_then(|lid| state.user_id(lid))
                    .unwrap_or(u32::MAX)
            } else {
                u32::MAX
            }
        } else {
            u32::MAX
        };

        push_mouse_event(state, button, x, y, pixel_x, pixel_y, modifiers, node_id);
    });
}

/// Perform a hit test at cell coordinates `(x, y)`.
///
/// Writes the hit path (deepest node first, then ancestors) as user-facing
/// `u32` node IDs into `out_ptr`.  Returns the number of IDs written.
///
/// # Safety
///
/// - `out_ptr` must point to a writable array of at least `max_depth` `u32` values.
#[no_mangle]
pub unsafe extern "C" fn hit_test(x: u16, y: u16, out_ptr: *mut u32, max_depth: u32) -> u32 {
    if out_ptr.is_null() || max_depth == 0 {
        return 0;
    }
    with_engine(|state| {
        let fx = f32::from(x);
        let fy = f32::from(y);

        let result = state.hit_tester.hit_test(&state.layout, fx, fy);
        let Ok(result) = result else {
            return 0;
        };

        if result.path.is_empty() {
            return 0;
        }

        // Write path in reverse order (deepest node first) for event bubbling.
        let out = unsafe { std::slice::from_raw_parts_mut(out_ptr, max_depth as usize) };
        let mut written: u32 = 0;
        for &layout_id in result.path.iter().rev() {
            if written >= max_depth {
                break;
            }
            if let Some(user_id) = state.user_id(layout_id) {
                out[written as usize] = user_id;
                written += 1;
            }
        }
        written
    })
}

/// Focus a specific node by its user-facing id.
///
/// Returns `1` if the node was focused, `0` otherwise.
#[no_mangle]
pub extern "C" fn focus(node_id: u32) -> u8 {
    with_engine(|state| {
        let Some(&layout_id) = state.node_map.get(&node_id) else {
            return 0;
        };
        let events = state.focus.focus_node(layout_id);
        for ev in &events {
            push_focus_event(state, ev);
        }
        u8::from(!events.is_empty())
    })
}

/// Blur the currently focused node.
///
/// Returns `1` if a node was blurred, `0` if nothing was focused.
#[no_mangle]
pub extern "C" fn blur() -> u8 {
    with_engine(|state| {
        if let Some(ev) = state.focus.blur() {
            push_focus_event(state, &ev);
            1
        } else {
            0
        }
    })
}

/// Return the user-facing id of the currently focused node, or `u32::MAX`.
#[no_mangle]
pub extern "C" fn get_focused_node() -> u32 {
    with_engine(|state| {
        state
            .focus
            .focused()
            .and_then(|lid| state.user_id(lid))
            .unwrap_or(u32::MAX)
    })
}

/// Mark a node as focusable with `tab_index = 0`.
#[no_mangle]
pub extern "C" fn set_focusable(node_id: u32, focusable: u8) {
    with_engine(|state| {
        if let Some(&layout_id) = state.node_map.get(&node_id) {
            if focusable != 0 {
                state.focus.set_meta(layout_id, FocusMeta::default());
            } else {
                state.focus.remove_meta(layout_id);
            }
        }
    });
}

/// Set the tab index for a node.
#[no_mangle]
pub extern "C" fn set_tab_index(node_id: u32, tab_index: i32) {
    with_engine(|state| {
        if let Some(&layout_id) = state.node_map.get(&node_id) {
            state.focus.set_meta(layout_id, FocusMeta { tab_index });
        }
    });
}

/// Enable or disable focus trapping on a node.
#[no_mangle]
pub extern "C" fn set_focus_trap(node_id: u32, enable: u8) {
    with_engine(|state| {
        if enable != 0 {
            if let Some(&layout_id) = state.node_map.get(&node_id) {
                state.focus.set_trap(layout_id);
            }
        } else {
            state.focus.clear_trap();
        }
    });
}

/// Push a focus/blur event into the event buffer.
fn push_focus_event(state: &mut EngineState, event: &crate::focus::FocusEvent) {
    match event {
        crate::focus::FocusEvent::Focus(lid) => {
            state.event_buffer.push(EVENT_FOCUS);
            let uid = state.user_id(*lid).unwrap_or(u32::MAX);
            state.event_buffer.extend_from_slice(&uid.to_le_bytes());
        }
        crate::focus::FocusEvent::Blur(lid) => {
            state.event_buffer.push(EVENT_BLUR);
            let uid = state.user_id(*lid).unwrap_or(u32::MAX);
            state.event_buffer.extend_from_slice(&uid.to_le_bytes());
        }
    }
}

// ---------------------------------------------------------------------------
// Layout queries
// ---------------------------------------------------------------------------

/// Write the computed layout (x, y, w, h) of a node into `out_ptr`.
///
/// # Safety
///
/// `out_ptr` must point to a writable array of at least 4 `f32` values.
#[no_mangle]
pub unsafe extern "C" fn get_layout(node_id: u32, out_ptr: *mut f32) {
    if out_ptr.is_null() {
        return;
    }
    with_engine(|state| {
        if let Some(&layout_id) = state.node_map.get(&node_id) {
            if let Ok(cl) = state.layout.get_layout(layout_id) {
                let out = unsafe { std::slice::from_raw_parts_mut(out_ptr, 4) };
                out[0] = cl.x;
                out[1] = cl.y;
                out[2] = cl.width;
                out[3] = cl.height;
            }
        }
    });
}

/// Write all computed layouts into `out_ptr`.  Each node occupies 5 floats:
/// `[node_id_as_f32, x, y, w, h]`.  Returns the number of nodes written.
///
/// # Safety
///
/// `out_ptr` must point to a writable array of at least `max_nodes * 5` `f32` values.
#[no_mangle]
pub unsafe extern "C" fn get_all_layouts(out_ptr: *mut f32, max_nodes: u32) -> u32 {
    if out_ptr.is_null() {
        return 0;
    }
    with_engine(|state| {
        let out = unsafe { std::slice::from_raw_parts_mut(out_ptr, max_nodes as usize * 5) };
        let mut written: u32 = 0;
        for (&node_id, &layout_id) in &state.node_map {
            if written >= max_nodes {
                break;
            }
            if let Ok(cl) = state.layout.get_layout(layout_id) {
                let base = written as usize * 5;
                #[allow(clippy::cast_precision_loss)]
                {
                    out[base] = node_id as f32;
                }
                out[base + 1] = cl.x;
                out[base + 2] = cl.y;
                out[base + 3] = cl.width;
                out[base + 4] = cl.height;
                written += 1;
            }
        }
        written
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use super::*;

    fn encode_create_node(node_id: u32, style_json: &str) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.push(OP_CREATE_NODE);
        buf.extend_from_slice(&node_id.to_le_bytes());
        let json_bytes = style_json.as_bytes();
        #[allow(clippy::cast_possible_truncation)]
        buf.extend_from_slice(&(json_bytes.len() as u16).to_le_bytes());
        buf.extend_from_slice(json_bytes);
        buf
    }

    fn encode_remove_node(node_id: u32) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.push(OP_REMOVE_NODE);
        buf.extend_from_slice(&node_id.to_le_bytes());
        buf
    }

    fn encode_append_child(parent_id: u32, child_id: u32) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.push(OP_APPEND_CHILD);
        buf.extend_from_slice(&parent_id.to_le_bytes());
        buf.extend_from_slice(&child_id.to_le_bytes());
        buf
    }

    fn encode_set_style(node_id: u32, style_json: &str) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.push(OP_SET_STYLE);
        buf.extend_from_slice(&node_id.to_le_bytes());
        let json_bytes = style_json.as_bytes();
        #[allow(clippy::cast_possible_truncation)]
        buf.extend_from_slice(&(json_bytes.len() as u16).to_le_bytes());
        buf.extend_from_slice(json_bytes);
        buf
    }

    fn encode_set_text(node_id: u32, text: &str) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.push(OP_SET_TEXT);
        buf.extend_from_slice(&node_id.to_le_bytes());
        let text_bytes = text.as_bytes();
        #[allow(clippy::cast_possible_truncation)]
        buf.extend_from_slice(&(text_bytes.len() as u16).to_le_bytes());
        buf.extend_from_slice(text_bytes);
        buf
    }

    fn encode_insert_before(parent_id: u32, child_id: u32, before_id: u32) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.push(OP_INSERT_BEFORE);
        buf.extend_from_slice(&parent_id.to_le_bytes());
        buf.extend_from_slice(&child_id.to_le_bytes());
        buf.extend_from_slice(&before_id.to_le_bytes());
        buf
    }

    fn setup() {
        // For tests, directly create engine state to avoid terminal side effects.
        let mut guard = ENGINE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut state = EngineState::new();
        // Route output to a Vec<u8> so tests don't write to the real terminal.
        state.output = Some(Vec::new());
        *guard = Some(state);
    }

    fn teardown() {
        RENDER_LOOP_RUNNING.store(false, Ordering::SeqCst);
        let mut guard = ENGINE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *guard = None;
    }

    /// Get the captured output bytes from the test engine.
    fn get_output() -> Vec<u8> {
        with_engine(|state| state.output.as_ref().unwrap_or(&Vec::new()).clone())
    }

    /// Clear the captured output.
    fn clear_output() {
        with_engine(|state| {
            if let Some(ref mut buf) = state.output {
                buf.clear();
            }
        });
    }

    #[test]
    #[serial]
    fn init_writes_capabilities_to_out_ptr() {
        let mut caps = InitResult {
            version_major: 0,
            version_minor: 0,
            version_patch: 0,
            batched_ffi: 0,
        };
        unsafe { init(&mut caps) };
        assert_eq!(caps.version_major, 0);
        assert_eq!(caps.version_minor, 1);
        assert_eq!(caps.version_patch, 0);
        assert_eq!(caps.batched_ffi, 1);
        teardown();
    }

    #[test]
    #[serial]
    fn create_and_remove_node() {
        setup();
        let buf = encode_create_node(1, r#"{"width":10,"height":5}"#);
        unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };

        with_engine(|state| {
            assert!(state.node_map.contains_key(&1));
        });

        let buf2 = encode_remove_node(1);
        unsafe { apply_mutations(buf2.as_ptr(), buf2.len() as u32) };

        with_engine(|state| {
            assert!(!state.node_map.contains_key(&1));
        });
        teardown();
    }

    #[test]
    #[serial]
    fn append_child_and_layout() {
        setup();
        let mut buf = Vec::new();
        buf.extend(encode_create_node(
            1,
            r#"{"width":80,"height":24,"flexDirection":"column"}"#,
        ));
        buf.extend(encode_create_node(2, r#"{"height":10}"#));
        buf.extend(encode_append_child(1, 2));
        unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };

        render_frame();

        let mut out = [0.0_f32; 4];
        unsafe { get_layout(2, out.as_mut_ptr()) };
        assert!(
            (out[3] - 10.0).abs() < f32::EPSILON,
            "height should be 10, got {}",
            out[3]
        );
        teardown();
    }

    #[test]
    #[serial]
    fn set_style_updates_node() {
        setup();
        let mut buf = Vec::new();
        buf.extend(encode_create_node(1, r#"{"width":80,"height":24}"#));
        buf.extend(encode_create_node(2, r#"{"width":10,"height":5}"#));
        buf.extend(encode_append_child(1, 2));
        unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };
        render_frame();

        let mut out = [0.0_f32; 4];
        unsafe { get_layout(2, out.as_mut_ptr()) };
        assert!((out[2] - 10.0).abs() < f32::EPSILON);

        let update = encode_set_style(2, r#"{"width":20,"height":5}"#);
        unsafe { apply_mutations(update.as_ptr(), update.len() as u32) };
        render_frame();

        unsafe { get_layout(2, out.as_mut_ptr()) };
        assert!(
            (out[2] - 20.0).abs() < f32::EPSILON,
            "width should be 20, got {}",
            out[2]
        );
        teardown();
    }

    #[test]
    #[serial]
    fn set_text_stores_content() {
        setup();
        let mut buf = Vec::new();
        buf.extend(encode_create_node(1, r#"{"width":80,"height":24}"#));
        buf.extend(encode_set_text(1, "Hello world"));
        unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };

        with_engine(|state| {
            assert_eq!(state.text_content.get(&1).unwrap(), "Hello world");
        });
        teardown();
    }

    #[test]
    #[serial]
    fn insert_before_adds_child() {
        setup();
        let mut buf = Vec::new();
        buf.extend(encode_create_node(1, r#"{"width":80,"height":24}"#));
        buf.extend(encode_create_node(2, r#"{"height":5}"#));
        buf.extend(encode_create_node(3, r#"{"height":5}"#));
        buf.extend(encode_append_child(1, 2));
        buf.extend(encode_insert_before(1, 3, 2));
        unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };

        with_engine(|state| {
            assert!(state.node_map.contains_key(&2));
            assert!(state.node_map.contains_key(&3));
        });
        teardown();
    }

    #[test]
    #[serial]
    fn get_all_layouts_returns_count() {
        setup();
        let mut buf = Vec::new();
        buf.extend(encode_create_node(
            1,
            r#"{"width":80,"height":24,"flexDirection":"column"}"#,
        ));
        buf.extend(encode_create_node(2, r#"{"height":10}"#));
        buf.extend(encode_create_node(3, r#"{"height":5}"#));
        buf.extend(encode_append_child(1, 2));
        buf.extend(encode_append_child(1, 3));
        unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };
        render_frame();

        let mut out = [0.0_f32; 15];
        let count = unsafe { get_all_layouts(out.as_mut_ptr(), 3) };
        assert_eq!(count, 3);
        teardown();
    }

    #[test]
    #[serial]
    fn empty_mutation_buffer_is_noop() {
        setup();
        unsafe { apply_mutations(std::ptr::null(), 0) };
        with_engine(|state| {
            assert!(state.node_map.is_empty());
        });
        teardown();
    }

    #[test]
    #[serial]
    fn request_render_sets_dirty() {
        setup();
        with_engine(|state| {
            assert!(!state.dirty);
        });
        request_render();
        with_engine(|state| {
            assert!(state.dirty);
        });
        teardown();
    }

    #[test]
    #[serial]
    fn keyboard_event_encoding() {
        setup();
        with_engine(|state| {
            push_keyboard_event(state, 65, 0b0000_0001, 1);
        });
        with_engine(|state| {
            assert_eq!(state.event_buffer.len(), 7);
            assert_eq!(state.event_buffer[0], EVENT_KEYBOARD);
            let key_code = u32::from_le_bytes(state.event_buffer[1..5].try_into().unwrap());
            assert_eq!(key_code, 65);
            assert_eq!(state.event_buffer[5], 0b0000_0001);
            assert_eq!(state.event_buffer[6], 1);
        });
        teardown();
    }

    #[test]
    #[serial]
    fn mouse_event_encoding() {
        setup();
        with_engine(|state| {
            push_mouse_event(state, 0, 10, 20, 80, 160, 0, 42);
        });
        with_engine(|state| {
            assert_eq!(state.event_buffer.len(), 15);
            assert_eq!(state.event_buffer[0], EVENT_MOUSE);
        });
        teardown();
    }

    #[test]
    #[serial]
    fn resize_event_encoding() {
        setup();
        with_engine(|state| {
            push_resize_event(state, 120, 40, 960, 640);
        });
        with_engine(|state| {
            assert_eq!(state.event_buffer.len(), 9);
            assert_eq!(state.event_buffer[0], EVENT_RESIZE);
            let cols = u16::from_le_bytes(state.event_buffer[1..3].try_into().unwrap());
            assert_eq!(cols, 120);
        });
        teardown();
    }

    #[test]
    #[serial]
    fn batched_mutations_in_single_buffer() {
        setup();
        let mut buf = Vec::new();
        buf.extend(encode_create_node(10, r#"{"width":80,"height":24}"#));
        buf.extend(encode_create_node(20, r#"{"width":40,"height":12}"#));
        buf.extend(encode_append_child(10, 20));
        buf.extend(encode_set_text(20, "batch test"));

        unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };

        with_engine(|state| {
            assert!(state.node_map.contains_key(&10));
            assert!(state.node_map.contains_key(&20));
            assert_eq!(state.text_content.get(&20).unwrap(), "batch test");
        });
        teardown();
    }

    #[test]
    #[serial]
    fn json_style_parsing() {
        let style =
            parse_style_json(br#"{"width":42,"height":10,"flexGrow":2,"flexDirection":"column"}"#);
        if let crate::layout::Dim::Cells(w) = style.width {
            assert!((w - 42.0).abs() < f32::EPSILON);
        } else {
            panic!("expected Cells");
        }
        if let crate::layout::DisplayMode::Flex(ref flex) = style.display {
            assert!((flex.grow - 2.0).abs() < f32::EPSILON);
            assert_eq!(flex.direction, crate::layout::FlexDir::Column);
        } else {
            panic!("expected Flex");
        }
    }

    #[test]
    #[serial]
    fn render_frame_clears_dirty() {
        setup();
        let buf = encode_create_node(1, r#"{"width":80,"height":24}"#);
        unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };
        with_engine(|state| {
            assert!(state.dirty);
        });
        render_frame();
        with_engine(|state| {
            assert!(!state.dirty);
        });
        teardown();
    }

    // -- Input FFI tests --

    #[test]
    #[serial]
    fn push_key_event_adds_to_buffer() {
        setup();
        push_key_event(65, 0b0000_0001, 1);
        with_engine(|state| {
            assert_eq!(state.event_buffer.len(), 7);
            assert_eq!(state.event_buffer[0], EVENT_KEYBOARD);
        });
        teardown();
    }

    #[test]
    #[serial]
    fn focus_and_blur_lifecycle() {
        setup();
        let mut buf = Vec::new();
        buf.extend(encode_create_node(
            1,
            r#"{"width":80,"height":24,"flexDirection":"column"}"#,
        ));
        buf.extend(encode_create_node(2, r#"{"width":20,"height":10}"#));
        buf.extend(encode_append_child(1, 2));
        unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };

        set_focusable(2, 1);
        let result = focus(2);
        assert_eq!(result, 1);
        assert_eq!(get_focused_node(), 2);

        with_engine(|state| {
            assert!(!state.event_buffer.is_empty());
            assert_eq!(state.event_buffer[0], EVENT_FOCUS);
            let nid = u32::from_le_bytes(state.event_buffer[1..5].try_into().unwrap());
            assert_eq!(nid, 2);
        });

        with_engine(|state| {
            state.event_buffer.clear();
        });

        let result = blur();
        assert_eq!(result, 1);
        assert_eq!(get_focused_node(), u32::MAX);

        with_engine(|state| {
            assert!(!state.event_buffer.is_empty());
            assert_eq!(state.event_buffer[0], EVENT_BLUR);
        });

        teardown();
    }

    #[test]
    #[serial]
    fn focus_unfocusable_node_fails() {
        setup();
        let buf = encode_create_node(1, r#"{"width":80,"height":24}"#);
        unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };

        let result = focus(1);
        assert_eq!(result, 0);
        assert_eq!(get_focused_node(), u32::MAX);
        teardown();
    }

    #[test]
    #[serial]
    fn focus_nonexistent_node_fails() {
        setup();
        let result = focus(999);
        assert_eq!(result, 0);
        teardown();
    }

    #[test]
    #[serial]
    fn set_tab_index_configures_node() {
        setup();
        let mut buf = Vec::new();
        buf.extend(encode_create_node(
            1,
            r#"{"width":80,"height":24,"flexDirection":"column"}"#,
        ));
        buf.extend(encode_create_node(2, r#"{"width":20,"height":5}"#));
        buf.extend(encode_create_node(3, r#"{"width":20,"height":5}"#));
        buf.extend(encode_append_child(1, 2));
        buf.extend(encode_append_child(1, 3));
        unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };

        set_tab_index(2, 0);
        set_tab_index(3, 1);

        focus(3);
        assert_eq!(get_focused_node(), 3);

        teardown();
    }

    #[test]
    #[serial]
    fn set_focusable_false_removes_focusability() {
        setup();
        let buf = encode_create_node(1, r#"{"width":80,"height":24}"#);
        unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };

        set_focusable(1, 1);
        let result = focus(1);
        assert_eq!(result, 1);

        set_focusable(1, 0);
        assert_eq!(get_focused_node(), u32::MAX);

        let result = focus(1);
        assert_eq!(result, 0);

        teardown();
    }

    #[test]
    #[serial]
    fn set_focus_trap_and_clear() {
        setup();
        let mut buf = Vec::new();
        buf.extend(encode_create_node(
            1,
            r#"{"width":80,"height":24,"flexDirection":"column"}"#,
        ));
        buf.extend(encode_create_node(2, r#"{"width":40,"height":12}"#));
        buf.extend(encode_create_node(3, r#"{"width":20,"height":5}"#));
        buf.extend(encode_append_child(1, 2));
        buf.extend(encode_append_child(1, 3));
        unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };

        set_focusable(2, 1);
        set_focusable(3, 1);

        set_focus_trap(1, 1);
        with_engine(|state| {
            assert!(state.focus.trap_root().is_some());
        });

        set_focus_trap(1, 0);
        with_engine(|state| {
            assert!(state.focus.trap_root().is_none());
        });

        teardown();
    }

    #[test]
    #[serial]
    fn blur_when_nothing_focused_returns_zero() {
        setup();
        let result = blur();
        assert_eq!(result, 0);
        teardown();
    }

    #[test]
    #[serial]
    fn remove_node_clears_focus() {
        setup();
        let mut buf = Vec::new();
        buf.extend(encode_create_node(
            1,
            r#"{"width":80,"height":24,"flexDirection":"column"}"#,
        ));
        buf.extend(encode_create_node(2, r#"{"width":20,"height":10}"#));
        buf.extend(encode_append_child(1, 2));
        unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };

        set_focusable(2, 1);
        focus(2);
        assert_eq!(get_focused_node(), 2);

        let remove_buf = encode_remove_node(2);
        unsafe { apply_mutations(remove_buf.as_ptr(), remove_buf.len() as u32) };
        assert_eq!(get_focused_node(), u32::MAX);

        teardown();
    }

    #[test]
    #[serial]
    fn push_mouse_event_with_hit_test_no_root() {
        setup();
        push_mouse_event_with_hit_test(0, 10, 20, 80, 160, 0);
        with_engine(|state| {
            assert_eq!(state.event_buffer.len(), 15);
            assert_eq!(state.event_buffer[0], EVENT_MOUSE);
            let node_id = u32::from_le_bytes(state.event_buffer[11..15].try_into().unwrap());
            assert_eq!(node_id, u32::MAX);
        });
        teardown();
    }

    #[test]
    #[serial]
    fn push_mouse_event_with_hit_test_hits_node() {
        setup();
        let mut buf = Vec::new();
        buf.extend(encode_create_node(
            1,
            r#"{"width":80,"height":24,"flexDirection":"column"}"#,
        ));
        buf.extend(encode_create_node(2, r#"{"width":20,"height":10}"#));
        buf.extend(encode_append_child(1, 2));
        unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };
        render_frame();

        push_mouse_event_with_hit_test(0, 5, 5, 40, 40, 0);
        with_engine(|state| {
            assert_eq!(state.event_buffer.len(), 15);
            let node_id = u32::from_le_bytes(state.event_buffer[11..15].try_into().unwrap());
            assert_eq!(node_id, 2);
        });
        teardown();
    }

    #[test]
    #[serial]
    fn focus_event_encoding() {
        setup();
        let buf = encode_create_node(1, r#"{"width":80,"height":24}"#);
        unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };
        set_focusable(1, 1);
        with_engine(|state| {
            state.event_buffer.clear();
        });

        focus(1);
        with_engine(|state| {
            assert_eq!(state.event_buffer.len(), 5);
            assert_eq!(state.event_buffer[0], EVENT_FOCUS);
            let nid = u32::from_le_bytes(state.event_buffer[1..5].try_into().unwrap());
            assert_eq!(nid, 1);
        });

        teardown();
    }

    // -- Render pipeline tests --

    #[test]
    #[serial]
    fn render_frame_produces_ansi_output_with_styles() {
        setup();
        let mut buf = Vec::new();
        buf.extend(encode_create_node(
            1,
            r#"{"width":80,"height":24,"flexDirection":"column"}"#,
        ));
        buf.extend(encode_create_node(
            2,
            r##"{"width":10,"height":3,"backgroundColor":"#FF0000","color":"#00FF00"}"##,
        ));
        buf.extend(encode_append_child(1, 2));
        buf.extend(encode_set_text(2, "Hi"));
        unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };

        render_frame();

        let output = get_output();
        let output_str = String::from_utf8_lossy(&output);
        // Should contain ANSI escape sequences for the red background.
        assert!(
            !output.is_empty(),
            "render_frame should produce output when nodes have styles"
        );
        // Should contain the cursor-positioning (CUP) sequence.
        assert!(
            output_str.contains('H'),
            "output should contain CUP sequence"
        );
        // Should contain the text content.
        assert!(output_str.contains("Hi"), "output should contain text 'Hi'");
        // Should contain RGB color codes for red bg (48;2;255;0;0).
        assert!(
            output_str.contains("48;2;255;0;0"),
            "output should contain red background SGR: {output_str}"
        );
        // Should contain RGB color codes for green fg (38;2;0;255;0).
        assert!(
            output_str.contains("38;2;0;255;0"),
            "output should contain green foreground SGR: {output_str}"
        );

        teardown();
    }

    #[test]
    #[serial]
    fn unchanged_frames_produce_no_diff_after_swap() {
        setup();
        let mut buf = Vec::new();
        buf.extend(encode_create_node(
            1,
            r#"{"width":80,"height":24,"flexDirection":"column"}"#,
        ));
        buf.extend(encode_create_node(
            2,
            r##"{"width":5,"height":2,"backgroundColor":"#0000FF"}"##,
        ));
        buf.extend(encode_append_child(1, 2));
        buf.extend(encode_set_text(2, "AB"));
        unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };

        // First render — should produce output.
        render_frame();
        let first_output = get_output();
        assert!(
            !first_output.is_empty(),
            "first render should produce output"
        );

        // Clear captured output.
        clear_output();

        // Mark dirty again with the exact same content.
        request_render();
        render_frame();

        let second_output = get_output();
        // In test mode (captured output), full_render is used so output is always
        // produced for complete screen state. The second render should produce
        // identical output to the first.
        assert!(
            !second_output.is_empty(),
            "second render in test mode should produce full output"
        );
        assert_eq!(
            first_output, second_output,
            "unchanged content should produce identical output"
        );

        teardown();
    }

    #[test]
    #[serial]
    fn hex_color_parsing_rrggbb() {
        let color = parse_hex_color("#FF8800");
        assert_eq!(color, Some(Color::Rgb(255, 136, 0)));
    }

    #[test]
    #[serial]
    fn hex_color_parsing_rgb_short() {
        let color = parse_hex_color("#F80");
        assert_eq!(color, Some(Color::Rgb(0xFF, 0x88, 0x00)));
    }

    #[test]
    #[serial]
    fn hex_color_parsing_invalid() {
        assert_eq!(parse_hex_color("not-a-color"), None);
        assert_eq!(parse_hex_color("#GG0000"), None);
        assert_eq!(parse_hex_color("#1234"), None);
    }

    #[test]
    #[serial]
    fn render_frame_skipped_when_not_dirty() {
        setup();
        let mut buf = Vec::new();
        buf.extend(encode_create_node(
            1,
            r##"{"width":80,"height":24,"backgroundColor":"#FF0000"}"##,
        ));
        unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };

        // First render (dirty from mutation).
        render_frame();
        clear_output();

        // render_frame without re-dirtying — dirty was cleared.
        render_frame();
        let output = get_output();
        assert!(
            output.is_empty(),
            "render_frame should skip painting when not dirty"
        );

        teardown();
    }

    #[test]
    #[serial]
    fn visual_style_parsed_from_json() {
        let vs = parse_visual_style_json(
            br##"{"backgroundColor":"#112233","color":"#AABBCC","width":10}"##,
        );
        assert_eq!(vs.bg, Some(Color::Rgb(0x11, 0x22, 0x33)));
        assert_eq!(vs.fg, Some(Color::Rgb(0xAA, 0xBB, 0xCC)));
    }

    // -- Hit-test FFI tests --

    #[test]
    #[serial]
    fn hit_test_returns_correct_node() {
        setup();
        let mut buf = Vec::new();
        buf.extend(encode_create_node(
            1,
            r#"{"width":80,"height":24,"flexDirection":"row"}"#,
        ));
        buf.extend(encode_create_node(2, r#"{"width":20,"height":10}"#));
        buf.extend(encode_create_node(3, r#"{"width":20,"height":10}"#));
        buf.extend(encode_append_child(1, 2));
        buf.extend(encode_append_child(1, 3));
        unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };
        render_frame();

        let mut out = [0_u32; 16];
        let count = unsafe { hit_test(5, 5, out.as_mut_ptr(), 16) };
        assert!(count >= 1, "should hit at least one node, got {count}");
        // Deepest node first — should be child 2.
        assert_eq!(out[0], 2, "deepest hit should be node 2");

        teardown();
    }

    #[test]
    #[serial]
    fn hit_test_empty_area_returns_zero() {
        setup();
        let mut buf = Vec::new();
        buf.extend(encode_create_node(1, r#"{"width":10,"height":10}"#));
        unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };
        render_frame();

        // Hit outside the root node bounds.
        let mut out = [0_u32; 16];
        let count = unsafe { hit_test(50, 50, out.as_mut_ptr(), 16) };
        assert_eq!(count, 0, "should return 0 for empty area");

        teardown();
    }

    #[test]
    #[serial]
    fn hit_test_path_includes_ancestors() {
        setup();
        let mut buf = Vec::new();
        buf.extend(encode_create_node(
            1,
            r#"{"width":80,"height":24,"flexDirection":"column"}"#,
        ));
        buf.extend(encode_create_node(2, r#"{"width":20,"height":10}"#));
        buf.extend(encode_append_child(1, 2));
        unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };
        render_frame();

        let mut out = [0_u32; 16];
        let count = unsafe { hit_test(5, 5, out.as_mut_ptr(), 16) };
        assert_eq!(count, 2, "path should include child + root");
        // Deepest first: child 2, then root 1.
        assert_eq!(out[0], 2);
        assert_eq!(out[1], 1);

        teardown();
    }

    #[test]
    #[serial]
    fn hit_test_grid_rebuilt_after_style_change() {
        setup();
        let mut buf = Vec::new();
        buf.extend(encode_create_node(
            1,
            r#"{"width":80,"height":24,"flexDirection":"row"}"#,
        ));
        buf.extend(encode_create_node(2, r#"{"width":20,"height":10}"#));
        buf.extend(encode_append_child(1, 2));
        unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };
        render_frame();

        // Initially node 2 is at x=0..20. Hit at x=5 should return node 2.
        let mut out = [0_u32; 16];
        let count = unsafe { hit_test(5, 5, out.as_mut_ptr(), 16) };
        assert!(count >= 1);
        assert_eq!(out[0], 2);

        // Move node 2 to be wider so x=25 is now inside it.
        let update = encode_set_style(2, r#"{"width":40,"height":10}"#);
        unsafe { apply_mutations(update.as_ptr(), update.len() as u32) };
        render_frame();

        let count2 = unsafe { hit_test(25, 5, out.as_mut_ptr(), 16) };
        assert!(count2 >= 1, "after style change, grid should be rebuilt");
        assert_eq!(out[0], 2, "node 2 should now cover x=25");

        teardown();
    }

    // -- Test-mode FFI tests --

    #[test]
    #[serial]
    fn init_test_mode_creates_engine_with_custom_size() {
        let mut caps = InitResult {
            version_major: 0,
            version_minor: 0,
            version_patch: 0,
            batched_ffi: 0,
        };
        unsafe { init_test_mode(40, 10, &mut caps) };
        assert_eq!(caps.version_major, 0);
        assert_eq!(caps.version_minor, 1);
        assert_eq!(caps.batched_ffi, 1);
        with_engine(|state| {
            assert!((state.cols - 40.0).abs() < f32::EPSILON);
            assert!((state.rows - 10.0).abs() < f32::EPSILON);
            assert_eq!(state.double_buf.width(), 40);
            assert_eq!(state.double_buf.height(), 10);
            assert!(state.output.is_some());
        });
        teardown();
    }

    #[test]
    #[serial]
    fn get_rendered_output_copies_and_clears() {
        unsafe { init_test_mode(20, 5, std::ptr::null_mut()) };

        // Create a node with content to trigger output.
        let mut buf = Vec::new();
        buf.extend(encode_create_node(
            1,
            r##"{"width":20,"height":5,"backgroundColor":"#FF0000"}"##,
        ));
        buf.extend(encode_set_text(1, "AB"));
        unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };
        render_frame();

        // Read the output.
        let mut out = vec![0u8; 4096];
        let n = unsafe { get_rendered_output(out.as_mut_ptr(), 4096) };
        assert!(n > 0, "should have produced output");
        let output_str = std::str::from_utf8(&out[..n as usize]).unwrap_or("");
        assert!(output_str.contains("AB"), "output should contain text");

        // Second call should return 0 (buffer was cleared).
        let n2 = unsafe { get_rendered_output(out.as_mut_ptr(), 4096) };
        assert_eq!(n2, 0, "buffer should be empty after first read");

        teardown();
    }

    #[test]
    #[serial]
    fn shutdown_test_mode_drops_state() {
        unsafe { init_test_mode(20, 5, std::ptr::null_mut()) };
        shutdown_test_mode();
        let guard = ENGINE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert!(guard.is_none());
    }

    #[test]
    #[serial]
    fn double_buffer_resizes_on_cols_rows_change() {
        setup();
        let buf = encode_create_node(1, r#"{"width":80,"height":24}"#);
        unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };

        // Change terminal size.
        with_engine(|state| {
            state.cols = 40.0;
            state.rows = 10.0;
            state.dirty = true;
        });

        render_frame();

        with_engine(|state| {
            assert_eq!(state.double_buf.width(), 40);
            assert_eq!(state.double_buf.height(), 10);
        });

        teardown();
    }

    #[test]
    #[serial]
    fn text_ellipsis_truncates_with_ellipsis_char() {
        setup();

        let mut buf = encode_create_node(1, r#"{"width":80,"height":24,"flexDirection":"column"}"#);
        buf.extend(encode_create_node(
            2,
            r#"{"width":8,"height":1,"textOverflow":"ellipsis"}"#,
        ));
        buf.extend(encode_append_child(1, 2));
        buf.extend(encode_set_text(2, "Hello World"));
        unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };

        render_frame();

        with_engine(|state| {
            let back = state.double_buf.back();
            let mut rendered = String::new();
            for col in 0..8 {
                if let Some(cell) = back.get(0, col) {
                    rendered.push(cell.ch);
                }
            }
            assert_eq!(rendered, "Hello W\u{2026}");
        });

        teardown();
    }

    // -- Text decoration: underline, strikethrough, dim --

    #[test]
    fn parse_visual_style_underline_bool() {
        let json = br#"{"underline":true}"#;
        let vs = parse_visual_style_json(json);
        assert!(vs.underline);
    }

    #[test]
    fn parse_visual_style_strikethrough_bool() {
        let json = br#"{"strikethrough":true}"#;
        let vs = parse_visual_style_json(json);
        assert!(vs.strikethrough);
    }

    #[test]
    fn parse_visual_style_dim_bool() {
        let json = br#"{"dim":true}"#;
        let vs = parse_visual_style_json(json);
        assert!(vs.dim);
    }

    #[test]
    fn parse_visual_style_text_decoration_underline() {
        let json = br#"{"textDecoration":"underline"}"#;
        let vs = parse_visual_style_json(json);
        assert!(vs.underline);
    }

    #[test]
    fn parse_visual_style_text_decoration_line_through() {
        let json = br#"{"textDecoration":"line-through"}"#;
        let vs = parse_visual_style_json(json);
        assert!(vs.strikethrough);
    }

    #[test]
    #[serial]
    fn underline_text_emits_sgr4() {
        teardown();
        unsafe { init_test_mode(20, 3, std::ptr::null_mut()) };

        let mut buf = Vec::new();
        // root node
        buf.extend(encode_create_node(1, r#"{"width":20,"height":3}"#));
        // child text node with underline
        buf.extend(encode_create_node(
            2,
            r#"{"width":20,"height":1,"underline":true}"#,
        ));
        buf.extend(encode_set_text(2, "hello"));
        buf.extend(encode_append_child(1, 2));
        unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };
        render_frame();

        with_engine(|state| {
            let cell = state.double_buf.back().get(0, 0).unwrap();
            assert_eq!(cell.ch, 'h');
            assert!(cell.style.underline, "cell should have underline set");
            // Verify SGR 4 is emitted
            let sgr = cell.style.to_sgr();
            let sgr_str = String::from_utf8_lossy(&sgr);
            assert!(
                sgr_str.contains(";4"),
                "SGR should contain ;4 for underline, got: {sgr_str}"
            );
        });

        teardown();
    }

    #[test]
    #[serial]
    fn text_that_fits_is_not_ellipsized() {
        setup();

        let mut buf = encode_create_node(1, r#"{"width":80,"height":24,"flexDirection":"column"}"#);
        buf.extend(encode_create_node(
            2,
            r#"{"width":8,"height":1,"textOverflow":"ellipsis"}"#,
        ));
        buf.extend(encode_append_child(1, 2));
        buf.extend(encode_set_text(2, "Hi"));
        unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };

        render_frame();

        with_engine(|state| {
            let back = state.double_buf.back();
            assert_eq!(back.get(0, 0).unwrap().ch, 'H');
            assert_eq!(back.get(0, 1).unwrap().ch, 'i');
            assert_eq!(back.get(0, 2).unwrap().ch, ' ');
        });

        teardown();
    }

    #[test]
    #[serial]
    fn overflow_hidden_clips_child_background() {
        setup();

        let buf = encode_create_node(1, r#"{"width":10,"height":1,"overflow":"hidden"}"#);
        unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };

        let buf = encode_create_node(
            2,
            r##"{"width":20,"height":1,"backgroundColor":"#ff0000","flexShrink":0}"##,
        );
        unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };

        let buf = encode_append_child(1, 2);
        unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };

        with_engine(|state| {
            state.root_node = Some(1);
            state.dirty = true;
        });

        render_frame();

        with_engine(|state| {
            let back = state.double_buf.back();
            for col in 0..10 {
                let cell = back.get(0, col).unwrap();
                assert_eq!(
                    cell.style.bg,
                    Some(crate::ansi::Color::Rgb(255, 0, 0)),
                    "cell at col {col} should have red bg"
                );
            }
            for col in 10..20 {
                if let Some(cell) = back.get(0, col) {
                    assert_ne!(
                        cell.style.bg,
                        Some(crate::ansi::Color::Rgb(255, 0, 0)),
                        "cell at col {col} should NOT have red bg (clipped)"
                    );
                }
            }
        });

        teardown();
    }

    #[test]
    #[serial]
    fn border_round_produces_corners() {
        setup();
        let mut buf = Vec::new();
        buf.extend(encode_create_node(
            1,
            r#"{"width":80,"height":24,"flexDirection":"column"}"#,
        ));
        buf.extend(encode_create_node(
            2,
            r##"{"width":10,"height":5,"border":"round","borderColor":"#FF0000"}"##,
        ));
        buf.extend(encode_append_child(1, 2));
        unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };

        render_frame();

        // Check the back buffer for border characters.
        with_engine(|state| {
            let back = state.double_buf.back();
            // Top-left corner: round = ╭
            assert_eq!(
                back.get(0, 0).map(|c| c.ch),
                Some('\u{256d}'),
                "top-left should be ╭"
            );
            // Top-right corner: round = ╮
            assert_eq!(
                back.get(0, 9).map(|c| c.ch),
                Some('\u{256e}'),
                "top-right should be ╮"
            );
            // Bottom-left corner: round = ╰
            assert_eq!(
                back.get(4, 0).map(|c| c.ch),
                Some('\u{2570}'),
                "bottom-left should be ╰"
            );
            // Bottom-right corner: round = ╯
            assert_eq!(
                back.get(4, 9).map(|c| c.ch),
                Some('\u{256f}'),
                "bottom-right should be ╯"
            );
            // Top edge: ─
            assert_eq!(
                back.get(0, 1).map(|c| c.ch),
                Some('\u{2500}'),
                "top edge should be ─"
            );
            // Left edge: │
            assert_eq!(
                back.get(1, 0).map(|c| c.ch),
                Some('\u{2502}'),
                "left edge should be │"
            );
            // Border color should be red fg.
            let corner_style = &back.get(0, 0).unwrap().style;
            assert_eq!(
                corner_style.fg,
                Some(Color::Rgb(255, 0, 0)),
                "border fg should be red"
            );
        });

        teardown();
    }

    #[test]
    #[serial]
    fn strikethrough_text_emits_sgr9() {
        teardown();
        unsafe { init_test_mode(20, 3, std::ptr::null_mut()) };

        let mut buf = Vec::new();
        buf.extend(encode_create_node(1, r#"{"width":20,"height":3}"#));
        buf.extend(encode_create_node(
            2,
            r#"{"width":20,"height":1,"strikethrough":true}"#,
        ));
        buf.extend(encode_set_text(2, "hello"));
        buf.extend(encode_append_child(1, 2));
        unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };
        render_frame();

        with_engine(|state| {
            let cell = state.double_buf.back().get(0, 0).unwrap();
            assert_eq!(cell.ch, 'h');
            assert!(
                cell.style.strikethrough,
                "cell should have strikethrough set"
            );
            let sgr = cell.style.to_sgr();
            let sgr_str = String::from_utf8_lossy(&sgr);
            assert!(
                sgr_str.contains(";9"),
                "SGR should contain ;9 for strikethrough, got: {sgr_str}"
            );
        });

        teardown();
    }

    #[test]
    #[serial]
    fn dim_text_emits_sgr2() {
        teardown();
        unsafe { init_test_mode(20, 3, std::ptr::null_mut()) };

        let mut buf = Vec::new();
        buf.extend(encode_create_node(1, r#"{"width":20,"height":3}"#));
        buf.extend(encode_create_node(
            2,
            r#"{"width":20,"height":1,"dim":true}"#,
        ));
        buf.extend(encode_set_text(2, "hello"));
        buf.extend(encode_append_child(1, 2));
        unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };
        render_frame();

        with_engine(|state| {
            let cell = state.double_buf.back().get(0, 0).unwrap();
            assert_eq!(cell.ch, 'h');
            assert!(cell.style.dim, "cell should have dim set");
            let sgr = cell.style.to_sgr();
            let sgr_str = String::from_utf8_lossy(&sgr);
            assert!(
                sgr_str.contains(";2"),
                "SGR should contain ;2 for dim, got: {sgr_str}"
            );
        });

        teardown();
    }
}
