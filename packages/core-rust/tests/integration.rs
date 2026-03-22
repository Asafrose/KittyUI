//! Integration tests for the KittyUI core FFI bridge.
//!
//! These tests exercise the full pipeline through the public C ABI surface:
//! mutation encoding → apply_mutations → render_frame → layout queries.
//!
//! Because the engine uses global state, all tests are serialised with `serial_test`.

use std::sync::Mutex;

use serial_test::serial;

// Re-export the FFI functions from the crate root (they are `pub extern "C"`).
use kittyui_core::ffi_bridge::InitResult;

// ---------------------------------------------------------------------------
// Helpers — binary mutation encoders (mirrors TS MutationEncoder)
// ---------------------------------------------------------------------------

const OP_CREATE_NODE: u8 = 1;
const OP_REMOVE_NODE: u8 = 2;
const OP_APPEND_CHILD: u8 = 3;
const OP_INSERT_BEFORE: u8 = 4;
const OP_SET_STYLE: u8 = 5;
const OP_SET_TEXT: u8 = 6;

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

fn encode_insert_before(parent_id: u32, child_id: u32, before_id: u32) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.push(OP_INSERT_BEFORE);
    buf.extend_from_slice(&parent_id.to_le_bytes());
    buf.extend_from_slice(&child_id.to_le_bytes());
    buf.extend_from_slice(&before_id.to_le_bytes());
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

// ---------------------------------------------------------------------------
// FFI wrappers (extern "C" functions from the cdylib)
// ---------------------------------------------------------------------------

extern "C" {
    fn init(out_ptr: *mut InitResult);
    fn shutdown();
    fn apply_mutations(buffer_ptr: *const u8, buffer_len: u32);
    fn render_frame();
    fn request_render();
    fn start_render_loop(fps: u32);
    fn stop_render_loop();
    fn get_layout(node_id: u32, out_ptr: *mut f32);
    fn get_all_layouts(out_ptr: *mut f32, max_nodes: u32) -> u32;
    fn push_key_event(key_code: u32, modifiers: u8, event_type: u8);
    fn push_mouse_event_with_hit_test(
        button: u8,
        x: u16,
        y: u16,
        pixel_x: u16,
        pixel_y: u16,
        modifiers: u8,
    );
    fn focus(node_id: u32) -> u8;
    fn blur() -> u8;
    fn get_focused_node() -> u32;
    fn set_focusable(node_id: u32, focusable: u8);
    fn set_tab_index(node_id: u32, tab_index: i32);
    fn set_focus_trap(node_id: u32, enable: u8);
    fn hit_test(x: u16, y: u16, out_ptr: *mut u32, max_depth: u32) -> u32;
}

fn setup() {
    unsafe { init(std::ptr::null_mut()) };
}

fn teardown() {
    unsafe { shutdown() };
}

// Use a global mutex to serialise tests that touch the ENGINE global.
// serial_test handles this, but we keep a guard for extra safety.
static TEST_MUTEX: Mutex<()> = Mutex::new(());

// =========================================================================
// Full pipeline tests
// =========================================================================

#[test]
#[serial]
fn full_pipeline_create_render_get_layout() {
    let _guard = TEST_MUTEX.lock();
    setup();

    let mut buf = Vec::new();
    buf.extend(encode_create_node(
        1,
        r#"{"width":80,"height":24,"flexDirection":"column"}"#,
    ));
    buf.extend(encode_create_node(2, r#"{"width":40,"height":10}"#));
    buf.extend(encode_append_child(1, 2));
    unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };

    unsafe { render_frame() };

    let mut out = [0.0_f32; 4];
    unsafe { get_layout(2, out.as_mut_ptr()) };
    assert!((out[0] - 0.0).abs() < f32::EPSILON, "x should be 0");
    assert!((out[1] - 0.0).abs() < f32::EPSILON, "y should be 0");
    assert!(
        (out[2] - 40.0).abs() < f32::EPSILON,
        "width should be 40, got {}",
        out[2]
    );
    assert!(
        (out[3] - 10.0).abs() < f32::EPSILON,
        "height should be 10, got {}",
        out[3]
    );

    teardown();
}

// =========================================================================
// Layout tests
// =========================================================================

#[test]
#[serial]
fn column_layout_stacks_children_vertically() {
    let _guard = TEST_MUTEX.lock();
    setup();

    let mut buf = Vec::new();
    buf.extend(encode_create_node(
        1,
        r#"{"width":80,"height":24,"flexDirection":"column"}"#,
    ));
    buf.extend(encode_create_node(2, r#"{"height":6}"#));
    buf.extend(encode_create_node(3, r#"{"height":8}"#));
    buf.extend(encode_append_child(1, 2));
    buf.extend(encode_append_child(1, 3));
    unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };
    unsafe { render_frame() };

    let mut out2 = [0.0_f32; 4];
    let mut out3 = [0.0_f32; 4];
    unsafe { get_layout(2, out2.as_mut_ptr()) };
    unsafe { get_layout(3, out3.as_mut_ptr()) };

    // Child 2 at y=0, child 3 at y=6
    assert!(
        (out2[1] - 0.0).abs() < f32::EPSILON,
        "child 2 y should be 0"
    );
    assert!(
        (out3[1] - 6.0).abs() < f32::EPSILON,
        "child 3 y should be 6, got {}",
        out3[1]
    );
    assert!(
        (out2[3] - 6.0).abs() < f32::EPSILON,
        "child 2 height should be 6"
    );
    assert!(
        (out3[3] - 8.0).abs() < f32::EPSILON,
        "child 3 height should be 8"
    );

    teardown();
}

#[test]
#[serial]
fn row_layout_stacks_children_horizontally() {
    let _guard = TEST_MUTEX.lock();
    setup();

    let mut buf = Vec::new();
    // Default flexDirection is "row"
    buf.extend(encode_create_node(1, r#"{"width":80,"height":24}"#));
    buf.extend(encode_create_node(2, r#"{"width":20,"height":10}"#));
    buf.extend(encode_create_node(3, r#"{"width":30,"height":10}"#));
    buf.extend(encode_append_child(1, 2));
    buf.extend(encode_append_child(1, 3));
    unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };
    unsafe { render_frame() };

    let mut out2 = [0.0_f32; 4];
    let mut out3 = [0.0_f32; 4];
    unsafe { get_layout(2, out2.as_mut_ptr()) };
    unsafe { get_layout(3, out3.as_mut_ptr()) };

    assert!(
        (out2[0] - 0.0).abs() < f32::EPSILON,
        "child 2 x should be 0"
    );
    assert!(
        (out3[0] - 20.0).abs() < f32::EPSILON,
        "child 3 x should be 20, got {}",
        out3[0]
    );

    teardown();
}

#[test]
#[serial]
fn nested_flexbox_layout() {
    let _guard = TEST_MUTEX.lock();
    setup();

    let mut buf = Vec::new();
    // Root: column, 80x24
    buf.extend(encode_create_node(
        1,
        r#"{"width":80,"height":24,"flexDirection":"column"}"#,
    ));
    // Row container: height 12
    buf.extend(encode_create_node(2, r#"{"height":12}"#));
    // Two children inside the row
    buf.extend(encode_create_node(3, r#"{"width":30,"height":12}"#));
    buf.extend(encode_create_node(4, r#"{"width":50,"height":12}"#));
    buf.extend(encode_append_child(1, 2));
    buf.extend(encode_append_child(2, 3));
    buf.extend(encode_append_child(2, 4));
    unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };
    unsafe { render_frame() };

    let mut out3 = [0.0_f32; 4];
    let mut out4 = [0.0_f32; 4];
    unsafe { get_layout(3, out3.as_mut_ptr()) };
    unsafe { get_layout(4, out4.as_mut_ptr()) };

    // Node 3 at x=0, node 4 at x=30 (row inside column)
    assert!((out3[0] - 0.0).abs() < f32::EPSILON);
    assert!(
        (out4[0] - 30.0).abs() < f32::EPSILON,
        "node 4 x should be 30, got {}",
        out4[0]
    );

    teardown();
}

#[test]
#[serial]
fn flex_grow_equal_distribution() {
    let _guard = TEST_MUTEX.lock();
    setup();

    let mut buf = Vec::new();
    buf.extend(encode_create_node(
        1,
        r#"{"width":80,"height":24,"flexDirection":"column"}"#,
    ));
    buf.extend(encode_create_node(2, r#"{"flexGrow":1}"#));
    buf.extend(encode_create_node(3, r#"{"flexGrow":1}"#));
    buf.extend(encode_append_child(1, 2));
    buf.extend(encode_append_child(1, 3));
    unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };
    unsafe { render_frame() };

    let mut out2 = [0.0_f32; 4];
    let mut out3 = [0.0_f32; 4];
    unsafe { get_layout(2, out2.as_mut_ptr()) };
    unsafe { get_layout(3, out3.as_mut_ptr()) };

    // Each child should get half the height (12)
    assert!(
        (out2[3] - 12.0).abs() < 0.01,
        "child 2 height should be 12, got {}",
        out2[3]
    );
    assert!(
        (out3[3] - 12.0).abs() < 0.01,
        "child 3 height should be 12, got {}",
        out3[3]
    );
    assert!(
        (out3[1] - 12.0).abs() < 0.01,
        "child 3 y should be 12, got {}",
        out3[1]
    );

    teardown();
}

#[test]
#[serial]
fn flex_grow_weighted_distribution() {
    let _guard = TEST_MUTEX.lock();
    setup();

    let mut buf = Vec::new();
    buf.extend(encode_create_node(
        1,
        r#"{"width":80,"height":24,"flexDirection":"column"}"#,
    ));
    buf.extend(encode_create_node(2, r#"{"flexGrow":1}"#));
    buf.extend(encode_create_node(3, r#"{"flexGrow":3}"#));
    buf.extend(encode_append_child(1, 2));
    buf.extend(encode_append_child(1, 3));
    unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };
    unsafe { render_frame() };

    let mut out2 = [0.0_f32; 4];
    let mut out3 = [0.0_f32; 4];
    unsafe { get_layout(2, out2.as_mut_ptr()) };
    unsafe { get_layout(3, out3.as_mut_ptr()) };

    // 1:3 ratio of 24 => 6 and 18
    assert!(
        (out2[3] - 6.0).abs() < 0.01,
        "child 2 height should be 6, got {}",
        out2[3]
    );
    assert!(
        (out3[3] - 18.0).abs() < 0.01,
        "child 3 height should be 18, got {}",
        out3[3]
    );

    teardown();
}

#[test]
#[serial]
fn padding_reduces_content_area() {
    let _guard = TEST_MUTEX.lock();
    setup();

    let mut buf = Vec::new();
    buf.extend(encode_create_node(
        1,
        r#"{"width":80,"height":24,"flexDirection":"column","padding":2}"#,
    ));
    buf.extend(encode_create_node(2, r#"{"flexGrow":1}"#));
    buf.extend(encode_append_child(1, 2));
    unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };
    unsafe { render_frame() };

    let mut out2 = [0.0_f32; 4];
    unsafe { get_layout(2, out2.as_mut_ptr()) };

    // With padding=2 on all sides, child position should be offset
    // and size should be reduced by 4 in each dimension
    assert!(
        (out2[0] - 2.0).abs() < 0.01,
        "child x should be 2 (padding), got {}",
        out2[0]
    );
    assert!(
        (out2[1] - 2.0).abs() < 0.01,
        "child y should be 2 (padding), got {}",
        out2[1]
    );
    // Width: 80 - 2*2 = 76
    assert!(
        (out2[2] - 76.0).abs() < 0.01,
        "child width should be 76, got {}",
        out2[2]
    );
    // Height: 24 - 2*2 = 20
    assert!(
        (out2[3] - 20.0).abs() < 0.01,
        "child height should be 20, got {}",
        out2[3]
    );

    teardown();
}

// =========================================================================
// Mutation tests
// =========================================================================

#[test]
#[serial]
fn set_style_updates_layout() {
    let _guard = TEST_MUTEX.lock();
    setup();

    let mut buf = Vec::new();
    buf.extend(encode_create_node(1, r#"{"width":80,"height":24}"#));
    buf.extend(encode_create_node(2, r#"{"width":10,"height":5}"#));
    buf.extend(encode_append_child(1, 2));
    unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };
    unsafe { render_frame() };

    let mut out = [0.0_f32; 4];
    unsafe { get_layout(2, out.as_mut_ptr()) };
    assert!((out[2] - 10.0).abs() < f32::EPSILON);

    let update = encode_set_style(2, r#"{"width":30,"height":5}"#);
    unsafe { apply_mutations(update.as_ptr(), update.len() as u32) };
    unsafe { render_frame() };

    unsafe { get_layout(2, out.as_mut_ptr()) };
    assert!(
        (out[2] - 30.0).abs() < f32::EPSILON,
        "width should be 30, got {}",
        out[2]
    );

    teardown();
}

#[test]
#[serial]
fn remove_node_cleanup() {
    let _guard = TEST_MUTEX.lock();
    setup();

    let mut buf = Vec::new();
    buf.extend(encode_create_node(
        1,
        r#"{"width":80,"height":24,"flexDirection":"column"}"#,
    ));
    buf.extend(encode_create_node(2, r#"{"height":10}"#));
    buf.extend(encode_append_child(1, 2));
    unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };

    // Now remove node 2
    let remove = encode_remove_node(2);
    unsafe { apply_mutations(remove.as_ptr(), remove.len() as u32) };

    // get_layout for removed node should return zeros
    let mut out = [99.0_f32; 4];
    unsafe { get_layout(2, out.as_mut_ptr()) };
    // Values should remain unchanged (99.0) since node doesn't exist
    assert!(
        (out[0] - 99.0).abs() < f32::EPSILON,
        "layout for removed node should not be written"
    );

    teardown();
}

#[test]
#[serial]
fn multiple_mutation_batches() {
    let _guard = TEST_MUTEX.lock();
    setup();

    // First batch: create root
    let buf1 = encode_create_node(1, r#"{"width":80,"height":24,"flexDirection":"column"}"#);
    unsafe { apply_mutations(buf1.as_ptr(), buf1.len() as u32) };

    // Second batch: add children
    let mut buf2 = Vec::new();
    buf2.extend(encode_create_node(2, r#"{"height":8}"#));
    buf2.extend(encode_create_node(3, r#"{"height":6}"#));
    buf2.extend(encode_append_child(1, 2));
    buf2.extend(encode_append_child(1, 3));
    unsafe { apply_mutations(buf2.as_ptr(), buf2.len() as u32) };

    unsafe { render_frame() };

    let mut out2 = [0.0_f32; 4];
    let mut out3 = [0.0_f32; 4];
    unsafe { get_layout(2, out2.as_mut_ptr()) };
    unsafe { get_layout(3, out3.as_mut_ptr()) };
    assert!((out2[3] - 8.0).abs() < f32::EPSILON);
    assert!((out3[3] - 6.0).abs() < f32::EPSILON);

    teardown();
}

// =========================================================================
// get_all_layouts
// =========================================================================

#[test]
#[serial]
fn get_all_layouts_returns_all_nodes() {
    let _guard = TEST_MUTEX.lock();
    setup();

    let mut buf = Vec::new();
    buf.extend(encode_create_node(
        1,
        r#"{"width":80,"height":24,"flexDirection":"column"}"#,
    ));
    for i in 2..=10 {
        buf.extend(encode_create_node(i, r#"{"height":2}"#));
        buf.extend(encode_append_child(1, i));
    }
    unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };
    unsafe { render_frame() };

    let mut out = [0.0_f32; 55]; // 11 nodes * 5
    let count = unsafe { get_all_layouts(out.as_mut_ptr(), 11) };
    // We created 10 nodes (1 root + 9 children)
    assert_eq!(count, 10, "should return 10 nodes");

    teardown();
}

// =========================================================================
// Stress test
// =========================================================================

#[test]
#[serial]
fn stress_test_500_nodes() {
    let _guard = TEST_MUTEX.lock();
    setup();

    let mut buf = Vec::new();
    buf.extend(encode_create_node(
        1,
        r#"{"width":80,"height":24,"flexDirection":"column"}"#,
    ));
    for i in 2..=500 {
        buf.extend(encode_create_node(i, r#"{"flexGrow":1}"#));
        buf.extend(encode_append_child(1, i));
    }
    unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };
    unsafe { render_frame() };

    let mut out = vec![0.0_f32; 501 * 5];
    let count = unsafe { get_all_layouts(out.as_mut_ptr(), 501) };
    assert_eq!(count, 500, "should return 500 nodes");

    // Verify root layout
    let mut root_out = [0.0_f32; 4];
    unsafe { get_layout(1, root_out.as_mut_ptr()) };
    assert!(
        (root_out[2] - 80.0).abs() < f32::EPSILON,
        "root width should be 80"
    );
    assert!(
        (root_out[3] - 24.0).abs() < f32::EPSILON,
        "root height should be 24"
    );

    teardown();
}

// =========================================================================
// Safety tests
// =========================================================================

#[test]
#[serial]
fn null_ptr_apply_mutations_is_safe() {
    let _guard = TEST_MUTEX.lock();
    setup();
    unsafe { apply_mutations(std::ptr::null(), 0) };
    // Should not crash
    teardown();
}

#[test]
#[serial]
fn null_ptr_get_layout_is_safe() {
    let _guard = TEST_MUTEX.lock();
    setup();
    unsafe { get_layout(1, std::ptr::null_mut()) };
    // Should not crash
    teardown();
}

#[test]
#[serial]
fn null_ptr_get_all_layouts_returns_zero() {
    let _guard = TEST_MUTEX.lock();
    setup();
    let count = unsafe { get_all_layouts(std::ptr::null_mut(), 10) };
    assert_eq!(count, 0);
    teardown();
}

#[test]
#[serial]
fn truncated_buffer_stops_gracefully() {
    let _guard = TEST_MUTEX.lock();
    setup();

    // Send a buffer that starts with CREATE_NODE but is truncated
    let buf: Vec<u8> = vec![OP_CREATE_NODE, 1, 0]; // too short
    unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };
    // Should not crash, engine should still be usable
    unsafe { render_frame() };

    teardown();
}

#[test]
#[serial]
fn unknown_opcode_stops_processing() {
    let _guard = TEST_MUTEX.lock();
    setup();

    // Valid create followed by unknown opcode
    let mut buf = encode_create_node(1, r#"{"width":80,"height":24}"#);
    buf.push(255); // unknown opcode
    buf.extend(encode_create_node(2, r#"{"width":10,"height":5}"#)); // should not be processed
    unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };

    // Node 1 should exist, node 2 should not
    let mut out1 = [0.0_f32; 4];
    unsafe { get_layout(1, out1.as_mut_ptr()) };
    // Just verify no crash — node 1 exists
    // Node 2's layout should not be set (remains zeros from init)
    let mut out2 = [99.0_f32; 4];
    unsafe { get_layout(2, out2.as_mut_ptr()) };
    assert!(
        (out2[0] - 99.0).abs() < f32::EPSILON,
        "node 2 should not have been created"
    );

    teardown();
}

#[test]
#[serial]
fn unicode_text_content() {
    let _guard = TEST_MUTEX.lock();
    setup();

    let mut buf = Vec::new();
    buf.extend(encode_create_node(1, r#"{"width":80,"height":24}"#));
    // Unicode text with emoji and CJK
    buf.extend(encode_set_text(1, "Hello \u{1F600} \u{4E16}\u{754C}"));
    unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };

    // Should not crash — we can't easily inspect text_content from integration tests
    // but the mutation should process without error
    unsafe { render_frame() };

    teardown();
}

// =========================================================================
// Event format verification
// =========================================================================

#[test]
#[serial]
fn keyboard_event_via_ffi() {
    let _guard = TEST_MUTEX.lock();
    setup();

    // Push a key event via the FFI function
    unsafe { push_key_event(65, 0b0000_0001, 1) };

    // The event is stored internally — we can verify it doesn't crash
    // and the render_frame will flush it
    unsafe { render_frame() };

    teardown();
}

#[test]
#[serial]
fn mouse_event_via_ffi_no_root() {
    let _guard = TEST_MUTEX.lock();
    setup();

    unsafe { push_mouse_event_with_hit_test(0, 10, 20, 80, 160, 0) };
    unsafe { render_frame() };
    // Should not crash even without any nodes

    teardown();
}

#[test]
#[serial]
fn focus_blur_events_via_ffi() {
    let _guard = TEST_MUTEX.lock();
    setup();

    let buf = encode_create_node(1, r#"{"width":80,"height":24}"#);
    unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };

    unsafe { set_focusable(1, 1) };
    let result = unsafe { focus(1) };
    assert_eq!(result, 1, "focus should succeed");
    assert_eq!(unsafe { get_focused_node() }, 1);

    let result = unsafe { blur() };
    assert_eq!(result, 1, "blur should succeed");
    assert_eq!(
        unsafe { get_focused_node() },
        u32::MAX,
        "no node should be focused"
    );

    teardown();
}

#[test]
#[serial]
fn focus_nonexistent_node_returns_zero() {
    let _guard = TEST_MUTEX.lock();
    setup();

    let result = unsafe { focus(999) };
    assert_eq!(result, 0);

    teardown();
}

#[test]
#[serial]
fn blur_with_nothing_focused_returns_zero() {
    let _guard = TEST_MUTEX.lock();
    setup();

    let result = unsafe { blur() };
    assert_eq!(result, 0);

    teardown();
}

// =========================================================================
// Init/shutdown cycling
// =========================================================================

#[test]
#[serial]
fn init_shutdown_cycling() {
    let _guard = TEST_MUTEX.lock();

    for _ in 0..10 {
        let mut caps = InitResult {
            version_major: 0,
            version_minor: 0,
            version_patch: 0,
            batched_ffi: 0,
        };
        unsafe { init(&mut caps) };
        assert_eq!(caps.version_major, 0);
        assert_eq!(caps.version_minor, 1);
        assert_eq!(caps.batched_ffi, 1);

        // Do some work
        let buf = encode_create_node(1, r#"{"width":80,"height":24}"#);
        unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };
        unsafe { render_frame() };

        unsafe { shutdown() };
    }
}

// =========================================================================
// Render loop start/stop
// =========================================================================

#[test]
#[serial]
fn render_loop_start_stop() {
    let _guard = TEST_MUTEX.lock();
    setup();

    unsafe { start_render_loop(30) };
    std::thread::sleep(std::time::Duration::from_millis(100));
    unsafe { stop_render_loop() };
    std::thread::sleep(std::time::Duration::from_millis(50));
    // Should not crash

    teardown();
}

#[test]
#[serial]
fn render_loop_double_start_is_idempotent() {
    let _guard = TEST_MUTEX.lock();
    setup();

    unsafe { start_render_loop(60) };
    unsafe { start_render_loop(60) }; // should be no-op
    std::thread::sleep(std::time::Duration::from_millis(50));
    unsafe { stop_render_loop() };
    std::thread::sleep(std::time::Duration::from_millis(50));

    teardown();
}

// =========================================================================
// Focus management
// =========================================================================

#[test]
#[serial]
fn set_tab_index_and_focus() {
    let _guard = TEST_MUTEX.lock();
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

    unsafe { set_tab_index(2, 0) };
    unsafe { set_tab_index(3, 1) };

    let result = unsafe { focus(3) };
    assert_eq!(result, 1);
    assert_eq!(unsafe { get_focused_node() }, 3);

    teardown();
}

#[test]
#[serial]
fn set_focusable_false_removes_focus() {
    let _guard = TEST_MUTEX.lock();
    setup();

    let buf = encode_create_node(1, r#"{"width":80,"height":24}"#);
    unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };

    unsafe { set_focusable(1, 1) };
    let result = unsafe { focus(1) };
    assert_eq!(result, 1);

    unsafe { set_focusable(1, 0) };
    assert_eq!(unsafe { get_focused_node() }, u32::MAX);

    teardown();
}

#[test]
#[serial]
fn focus_trap_set_and_clear() {
    let _guard = TEST_MUTEX.lock();
    setup();

    let mut buf = Vec::new();
    buf.extend(encode_create_node(
        1,
        r#"{"width":80,"height":24,"flexDirection":"column"}"#,
    ));
    buf.extend(encode_create_node(2, r#"{"width":20,"height":5}"#));
    buf.extend(encode_append_child(1, 2));
    unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };

    unsafe { set_focusable(2, 1) };
    unsafe { set_focus_trap(1, 1) };
    // Should not crash
    unsafe { set_focus_trap(1, 0) };

    teardown();
}

// =========================================================================
// Insert before
// =========================================================================

#[test]
#[serial]
fn insert_before_adds_child_to_tree() {
    let _guard = TEST_MUTEX.lock();
    setup();

    let mut buf = Vec::new();
    buf.extend(encode_create_node(
        1,
        r#"{"width":80,"height":24,"flexDirection":"column"}"#,
    ));
    buf.extend(encode_create_node(2, r#"{"height":5}"#));
    buf.extend(encode_create_node(3, r#"{"height":7}"#));
    buf.extend(encode_append_child(1, 2));
    buf.extend(encode_insert_before(1, 3, 2));
    unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };
    unsafe { render_frame() };

    // Both nodes should have valid layouts
    let mut out2 = [0.0_f32; 4];
    let mut out3 = [0.0_f32; 4];
    unsafe { get_layout(2, out2.as_mut_ptr()) };
    unsafe { get_layout(3, out3.as_mut_ptr()) };

    assert!(
        (out2[3] - 5.0).abs() < f32::EPSILON,
        "node 2 height should be 5"
    );
    assert!(
        (out3[3] - 7.0).abs() < f32::EPSILON,
        "node 3 height should be 7"
    );

    teardown();
}

// =========================================================================
// request_render
// =========================================================================

#[test]
#[serial]
fn request_render_and_render_frame() {
    let _guard = TEST_MUTEX.lock();
    setup();

    let buf = encode_create_node(1, r#"{"width":80,"height":24}"#);
    unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };

    unsafe { request_render() };
    unsafe { render_frame() };

    let mut out = [0.0_f32; 4];
    unsafe { get_layout(1, out.as_mut_ptr()) };
    assert!((out[2] - 80.0).abs() < f32::EPSILON);

    teardown();
}

// =========================================================================
// Hit testing
// =========================================================================

#[test]
#[serial]
fn hit_test_returns_correct_node() {
    let _guard = TEST_MUTEX.lock();
    setup();

    let mut buf = Vec::new();
    buf.extend(encode_create_node(
        1,
        r#"{"width":80,"height":24,"flexDirection":"column"}"#,
    ));
    buf.extend(encode_create_node(2, r#"{"width":40,"height":12}"#));
    buf.extend(encode_create_node(3, r#"{"width":40,"height":12}"#));
    buf.extend(encode_append_child(1, 2));
    buf.extend(encode_append_child(1, 3));
    unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };
    unsafe { render_frame() };

    let mut out = [0_u32; 16];
    let count = unsafe { hit_test(5, 5, out.as_mut_ptr(), 16) };
    assert!(count >= 1, "should hit at least one node, got {count}");
    // Deepest node first — child 2 occupies y=0..12.
    assert_eq!(out[0], 2, "deepest hit at (5,5) should be node 2");

    // Hit second child at y=15 (child 3 occupies y=12..24).
    let count2 = unsafe { hit_test(5, 15, out.as_mut_ptr(), 16) };
    assert!(count2 >= 1);
    assert_eq!(out[0], 3, "deepest hit at (5,15) should be node 3");

    teardown();
}

#[test]
#[serial]
fn hit_test_miss_returns_zero() {
    let _guard = TEST_MUTEX.lock();
    setup();

    let buf = encode_create_node(1, r#"{"width":10,"height":10}"#);
    unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };
    unsafe { render_frame() };

    // Coordinates well outside the 10x10 root.
    let mut out = [0_u32; 16];
    let count = unsafe { hit_test(50, 50, out.as_mut_ptr(), 16) };
    assert_eq!(count, 0, "hit test outside all nodes should return 0");

    teardown();
}

#[test]
#[serial]
fn hit_test_path_includes_ancestors() {
    let _guard = TEST_MUTEX.lock();
    setup();

    let mut buf = Vec::new();
    buf.extend(encode_create_node(
        1,
        r#"{"width":80,"height":24,"flexDirection":"column"}"#,
    ));
    buf.extend(encode_create_node(2, r#"{"width":40,"height":12}"#));
    buf.extend(encode_append_child(1, 2));
    unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };
    unsafe { render_frame() };

    let mut out = [0_u32; 16];
    let count = unsafe { hit_test(5, 5, out.as_mut_ptr(), 16) };
    assert_eq!(count, 2, "path should include child + root");
    // Deepest first: child 2, then root 1.
    assert_eq!(out[0], 2, "first in path should be deepest node (child)");
    assert_eq!(out[1], 1, "second in path should be ancestor (root)");

    teardown();
}
