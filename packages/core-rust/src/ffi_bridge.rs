//! Batched FFI bridge — coarse-grained C ABI surface for `bun:ffi`.
//!
//! Instead of many small FFI calls, JS batches tree mutations into a binary
//! buffer and sends them in a single `apply_mutations()` call.  Rust owns all
//! state (layout tree, text content, event queue, render loop).

use std::collections::HashMap;
use std::ffi::c_char;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use crate::layout::{LayoutNodeId, LayoutTree, NodeStyle};

// ---------------------------------------------------------------------------
// Op codes — must stay in sync with TS `MutationEncoder`
// ---------------------------------------------------------------------------

const OP_CREATE_NODE: u8 = 1;
const OP_REMOVE_NODE: u8 = 2;
const OP_APPEND_CHILD: u8 = 3;
const OP_INSERT_BEFORE: u8 = 4;
const OP_SET_STYLE: u8 = 5;
const OP_SET_TEXT: u8 = 6;

// ---------------------------------------------------------------------------
// Event types — must stay in sync with TS `EventDecoder`
// ---------------------------------------------------------------------------

const EVENT_KEYBOARD: u8 = 1;
const EVENT_MOUSE: u8 = 2;
const EVENT_RESIZE: u8 = 3;

// ---------------------------------------------------------------------------
// Global engine state
// ---------------------------------------------------------------------------

struct EngineState {
    layout: LayoutTree,
    /// Maps user-facing u32 node ids to Taffy `LayoutNodeId` handles.
    node_map: HashMap<u32, LayoutNodeId>,
    /// Text content per node (`node_id` to string).
    text_content: HashMap<u32, String>,
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
}

impl EngineState {
    fn new() -> Self {
        Self {
            layout: LayoutTree::new(),
            node_map: HashMap::new(),
            text_content: HashMap::new(),
            event_buffer: Vec::new(),
            root_node: None,
            dirty: false,
            cols: 80.0,
            rows: 24.0,
            event_callback: None,
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

/// Initialise the engine.  Returns a JSON capabilities string.
///
/// # Safety
///
/// Must be called exactly once before any other FFI function.
#[no_mangle]
pub extern "C" fn init() -> *const c_char {
    let mut guard = ENGINE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *guard = Some(EngineState::new());
    c"{\"version\":\"0.1.0\",\"batched_ffi\":true}".as_ptr()
}

/// Shut down the engine and release all resources.
#[no_mangle]
pub extern "C" fn shutdown() {
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
    if let Some(v) = json_extract_f32(s, "padding") {
        style.padding = [crate::layout::Dim::Cells(v); 4];
    }
    if let Some(v) = json_extract_f32(s, "margin") {
        style.margin = [crate::layout::Dim::Cells(v); 4];
    }
    if let Some(v) = json_extract_f32(s, "gap") {
        style.gap = [crate::layout::Dim::Cells(v); 2];
    }

    style
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
#[allow(clippy::too_many_lines)]
fn process_mutation(reader: &mut MutationReader<'_>, state: &mut EngineState) -> bool {
    let Some(op) = reader.read_u8() else {
        return false;
    };
    match op {
        OP_CREATE_NODE => {
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
            if let Ok(layout_id) = state.layout.add_leaf(&style) {
                state.node_map.insert(node_id, layout_id);
                if state.root_node.is_none() {
                    state.root_node = Some(node_id);
                }
            }
            state.dirty = true;
        }
        OP_REMOVE_NODE => {
            let Some(node_id) = reader.read_u32() else {
                return false;
            };
            if let Some(layout_id) = state.node_map.remove(&node_id) {
                let _ = state.layout.remove(layout_id);
            }
            state.text_content.remove(&node_id);
            state.dirty = true;
        }
        OP_APPEND_CHILD => {
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
            state.dirty = true;
        }
        OP_INSERT_BEFORE => {
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
            state.dirty = true;
        }
        OP_SET_STYLE => {
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
            if let Some(&layout_id) = state.node_map.get(&node_id) {
                let _ = state.layout.set_style(layout_id, &style);
            }
            state.dirty = true;
        }
        OP_SET_TEXT => {
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
            state.dirty = true;
        }
        _ => {
            return false;
        }
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

/// Run the full render pipeline: layout then flush events to JS.
#[no_mangle]
pub extern "C" fn render_frame() {
    with_engine(|state| {
        if let Some(root_id) = state.root_node {
            if let Some(&layout_id) = state.node_map.get(&root_id) {
                let _ = state.layout.compute(layout_id, state.cols, state.rows);
            }
        }
        state.dirty = false;

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
        init();
    }

    fn teardown() {
        shutdown();
    }

    #[test]
    #[serial]
    fn init_returns_capabilities_json() {
        let ptr = init();
        let cstr = unsafe { std::ffi::CStr::from_ptr(ptr) };
        let s = cstr.to_str().unwrap();
        assert!(s.contains("\"batched_ffi\":true"));
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
}
