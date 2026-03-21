//! Integration tests for the KittyUI FFI bridge.
//!
//! These tests exercise the `extern "C"` surface exactly as a foreign caller
//! (TypeScript via `bun:ffi`) would.  Because the FFI bridge uses a global
//! `static ENGINE: Mutex<Option<EngineState>>`, every test MUST be serialised
//! via `TEST_LOCK`.

// The crate is called `kittyui-core` in Cargo.toml, which Rust normalises to
// `kittyui_core` for `use` statements.
use kittyui_core::ffi_bridge::{
    apply_mutations, get_all_layouts, get_layout, init, render_frame, shutdown, start_render_loop,
    stop_render_loop,
};

// ---------------------------------------------------------------------------
// Serialisation lock — all tests share the global ENGINE
// ---------------------------------------------------------------------------

static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

macro_rules! serial_test {
    ($name:ident, $body:block) => {
        #[test]
        fn $name() {
            let _guard = TEST_LOCK
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            $body
        }
    };
}

// ---------------------------------------------------------------------------
// Helpers — setup / teardown
// ---------------------------------------------------------------------------

fn setup() {
    unsafe { init(std::ptr::null_mut()) };
}

fn teardown() {
    shutdown();
}

// ---------------------------------------------------------------------------
// Mutation encoding helpers (mirror TS MutationEncoder exactly)
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
// Layout query helpers
// ---------------------------------------------------------------------------

/// Read [x, y, w, h] for a single node.
fn read_layout(node_id: u32) -> [f32; 4] {
    let mut out = [0.0_f32; 4];
    unsafe { get_layout(node_id, out.as_mut_ptr()) };
    out
}

/// Read all layouts. Returns a Vec of (node_id, x, y, w, h) tuples.
fn read_all_layouts(max_nodes: u32) -> Vec<(u32, f32, f32, f32, f32)> {
    let mut buf = vec![0.0_f32; max_nodes as usize * 5];
    let count = unsafe { get_all_layouts(buf.as_mut_ptr(), max_nodes) };
    let mut results = Vec::new();
    for i in 0..count as usize {
        let base = i * 5;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let node_id = buf[base] as u32;
        results.push((
            node_id,
            buf[base + 1],
            buf[base + 2],
            buf[base + 3],
            buf[base + 4],
        ));
    }
    results
}

/// Build a complete tree from mutations buffer and render it.
fn apply_and_render(buf: &[u8]) {
    unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };
    render_frame();
}

/// Float comparison with epsilon tolerance.
fn assert_float_eq(actual: f32, expected: f32, label: &str) {
    assert!(
        (actual - expected).abs() < f32::EPSILON,
        "{label}: expected {expected}, got {actual}"
    );
}

// ---------------------------------------------------------------------------
// Test cases
// ---------------------------------------------------------------------------

serial_test!(full_pipeline_column_layout, {
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
    apply_and_render(&buf);

    let parent = read_layout(1);
    assert_float_eq(parent[0], 0.0, "parent x");
    assert_float_eq(parent[1], 0.0, "parent y");
    assert_float_eq(parent[2], 80.0, "parent w");
    assert_float_eq(parent[3], 24.0, "parent h");

    let child1 = read_layout(2);
    assert_float_eq(child1[0], 0.0, "child1 x");
    assert_float_eq(child1[1], 0.0, "child1 y");
    assert_float_eq(child1[2], 80.0, "child1 w");
    assert_float_eq(child1[3], 10.0, "child1 h");

    let child2 = read_layout(3);
    assert_float_eq(child2[0], 0.0, "child2 x");
    assert_float_eq(child2[1], 10.0, "child2 y");
    assert_float_eq(child2[2], 80.0, "child2 w");
    assert_float_eq(child2[3], 5.0, "child2 h");

    teardown();
});

serial_test!(full_pipeline_row_layout, {
    setup();

    // Default flex direction is row.
    let mut buf = Vec::new();
    buf.extend(encode_create_node(1, r#"{"width":80,"height":24}"#));
    buf.extend(encode_create_node(2, r#"{"width":30,"height":10}"#));
    buf.extend(encode_create_node(3, r#"{"width":30,"height":10}"#));
    buf.extend(encode_append_child(1, 2));
    buf.extend(encode_append_child(1, 3));
    apply_and_render(&buf);

    let child1 = read_layout(2);
    assert_float_eq(child1[0], 0.0, "child1 x");

    let child2 = read_layout(3);
    assert_float_eq(child2[0], 30.0, "child2 x");

    teardown();
});

serial_test!(nested_flexbox_3_levels, {
    setup();

    // Root: 80x24 column
    // Row container: flexGrow 1 (fills root height)
    // Two leaves inside row: w:30 each, side by side
    let mut buf = Vec::new();
    buf.extend(encode_create_node(
        1,
        r#"{"width":80,"height":24,"flexDirection":"column"}"#,
    ));
    buf.extend(encode_create_node(2, r#"{"flexGrow":1}"#)); // row container (default row direction)
    buf.extend(encode_create_node(3, r#"{"width":30}"#));
    buf.extend(encode_create_node(4, r#"{"width":30}"#));
    buf.extend(encode_append_child(1, 2));
    buf.extend(encode_append_child(2, 3));
    buf.extend(encode_append_child(2, 4));
    apply_and_render(&buf);

    let row = read_layout(2);
    assert_float_eq(row[2], 80.0, "row container width");

    let leaf1 = read_layout(3);
    let leaf2 = read_layout(4);
    assert_float_eq(leaf1[0], 0.0, "leaf1 x (relative to row)");
    assert_float_eq(leaf2[0], 30.0, "leaf2 x (relative to row)");

    teardown();
});

serial_test!(flex_grow_equal_distribution, {
    setup();

    let mut buf = Vec::new();
    buf.extend(encode_create_node(
        1,
        r#"{"width":80,"height":24,"flexDirection":"column"}"#,
    ));
    buf.extend(encode_create_node(2, r#"{"flexGrow":1}"#));
    buf.extend(encode_create_node(3, r#"{"flexGrow":1}"#));
    buf.extend(encode_create_node(4, r#"{"flexGrow":1}"#));
    buf.extend(encode_append_child(1, 2));
    buf.extend(encode_append_child(1, 3));
    buf.extend(encode_append_child(1, 4));
    apply_and_render(&buf);

    let c1 = read_layout(2);
    let c2 = read_layout(3);
    let c3 = read_layout(4);
    assert_float_eq(c1[3], 8.0, "child1 height (24/3)");
    assert_float_eq(c2[3], 8.0, "child2 height (24/3)");
    assert_float_eq(c3[3], 8.0, "child3 height (24/3)");

    teardown();
});

serial_test!(flex_grow_weighted, {
    setup();

    let mut buf = Vec::new();
    buf.extend(encode_create_node(
        1,
        r#"{"width":80,"height":24,"flexDirection":"column"}"#,
    ));
    buf.extend(encode_create_node(2, r#"{"flexGrow":2}"#));
    buf.extend(encode_create_node(3, r#"{"flexGrow":1}"#));
    buf.extend(encode_append_child(1, 2));
    buf.extend(encode_append_child(1, 3));
    apply_and_render(&buf);

    let a = read_layout(2);
    let b = read_layout(3);
    assert_float_eq(a[3], 16.0, "child A height (2/3 of 24)");
    assert_float_eq(b[3], 8.0, "child B height (1/3 of 24)");

    teardown();
});

serial_test!(padding_offsets_children, {
    setup();

    let mut buf = Vec::new();
    buf.extend(encode_create_node(
        1,
        r#"{"width":80,"height":24,"flexDirection":"column","padding":2}"#,
    ));
    buf.extend(encode_create_node(2, r#"{}"#));
    buf.extend(encode_append_child(1, 2));
    apply_and_render(&buf);

    let child = read_layout(2);
    assert_float_eq(child[0], 2.0, "child x offset by padding");
    assert_float_eq(child[1], 2.0, "child y offset by padding");
    // width = 80 - 2*2 = 76
    assert_float_eq(child[2], 76.0, "child width reduced by padding");

    teardown();
});

serial_test!(set_style_updates_layout, {
    setup();

    // Initial tree: parent column, child height 10.
    let mut buf = Vec::new();
    buf.extend(encode_create_node(
        1,
        r#"{"width":80,"height":24,"flexDirection":"column"}"#,
    ));
    buf.extend(encode_create_node(2, r#"{"height":10}"#));
    buf.extend(encode_append_child(1, 2));
    apply_and_render(&buf);

    let before = read_layout(2);
    assert_float_eq(before[3], 10.0, "initial height");

    // Update child height to 15.
    let update = encode_set_style(2, r#"{"height":15}"#);
    apply_and_render(&update);

    let after = read_layout(2);
    assert_float_eq(after[3], 15.0, "updated height");

    teardown();
});

serial_test!(multiple_batches_accumulate, {
    setup();

    // Batch 1: create parent.
    let batch1 = encode_create_node(1, r#"{"width":80,"height":24,"flexDirection":"column"}"#);
    unsafe { apply_mutations(batch1.as_ptr(), batch1.len() as u32) };

    // Batch 2: create child + appendChild.
    let mut batch2 = Vec::new();
    batch2.extend(encode_create_node(2, r#"{"height":10}"#));
    batch2.extend(encode_append_child(1, 2));
    unsafe { apply_mutations(batch2.as_ptr(), batch2.len() as u32) };

    render_frame();

    let parent = read_layout(1);
    assert_float_eq(parent[2], 80.0, "parent width");

    let child = read_layout(2);
    assert_float_eq(child[3], 10.0, "child height");

    teardown();
});

serial_test!(remove_node_cleanup, {
    setup();

    let mut buf = Vec::new();
    buf.extend(encode_create_node(
        1,
        r#"{"width":80,"height":24,"flexDirection":"column"}"#,
    ));
    buf.extend(encode_create_node(2, r#"{"height":10}"#));
    buf.extend(encode_append_child(1, 2));
    apply_and_render(&buf);

    // Verify 2 nodes.
    let layouts = read_all_layouts(10);
    assert_eq!(layouts.len(), 2, "should have 2 nodes before removal");

    // Remove child.
    let remove = encode_remove_node(2);
    apply_and_render(&remove);

    let layouts = read_all_layouts(10);
    assert_eq!(layouts.len(), 1, "should have 1 node after removal");

    teardown();
});

serial_test!(get_all_layouts_with_50_nodes, {
    setup();

    let mut buf = Vec::new();
    buf.extend(encode_create_node(
        0,
        r#"{"width":80,"height":24,"flexDirection":"column"}"#,
    ));
    for i in 1..=50 {
        buf.extend(encode_create_node(i, r#"{"flexGrow":1}"#));
        buf.extend(encode_append_child(0, i));
    }
    apply_and_render(&buf);

    let count = {
        let mut out = vec![0.0_f32; 51 * 5];
        unsafe { get_all_layouts(out.as_mut_ptr(), 51) }
    };
    assert_eq!(count, 51, "should return 51 nodes (1 parent + 50 children)");

    teardown();
});

serial_test!(stress_500_nodes, {
    setup();

    let mut buf = Vec::new();
    buf.extend(encode_create_node(
        0,
        r#"{"width":80,"height":24,"flexDirection":"column"}"#,
    ));
    for i in 1..=500 {
        buf.extend(encode_create_node(i, r#"{"flexGrow":1}"#));
        buf.extend(encode_append_child(0, i));
    }
    apply_and_render(&buf);

    let count = {
        let mut out = vec![0.0_f32; 501 * 5];
        unsafe { get_all_layouts(out.as_mut_ptr(), 501) }
    };
    assert_eq!(
        count, 501,
        "should return 501 nodes (1 parent + 500 children)"
    );

    teardown();
});

serial_test!(null_pointer_safety, {
    setup();

    // apply_mutations with null pointer should be a no-op.
    unsafe { apply_mutations(std::ptr::null(), 0) };

    // get_layout for unknown node should write zeros (buffer stays zeroed).
    let layout = read_layout(9999);
    assert_float_eq(layout[0], 0.0, "unknown node x");
    assert_float_eq(layout[1], 0.0, "unknown node y");
    assert_float_eq(layout[2], 0.0, "unknown node w");
    assert_float_eq(layout[3], 0.0, "unknown node h");

    // get_all_layouts with max_nodes=0 returns 0.
    let mut dummy = [0.0_f32; 1];
    let count = unsafe { get_all_layouts(dummy.as_mut_ptr(), 0) };
    assert_eq!(count, 0, "max_nodes=0 should return 0");

    teardown();
});

serial_test!(truncated_buffer, {
    setup();

    // Buffer starts with CREATE_NODE opcode but is truncated (missing node_id).
    let buf: Vec<u8> = vec![OP_CREATE_NODE, 0x01];
    unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };

    // Should not panic. No nodes created.
    let layouts = read_all_layouts(10);
    assert!(
        layouts.is_empty(),
        "truncated buffer should not create nodes"
    );

    teardown();
});

serial_test!(unknown_opcode, {
    setup();

    // Buffer with unknown opcode 255.  Processing should stop, no panic.
    let buf: Vec<u8> = vec![255, 0, 0, 0, 0];
    unsafe { apply_mutations(buf.as_ptr(), buf.len() as u32) };

    let layouts = read_all_layouts(10);
    assert!(layouts.is_empty(), "unknown opcode should not create nodes");

    teardown();
});

serial_test!(unicode_text_round_trip, {
    setup();

    let mut buf = Vec::new();
    buf.extend(encode_create_node(1, r#"{"width":80,"height":24}"#));
    buf.extend(encode_set_text(1, "Hello \u{1F30D}"));
    apply_and_render(&buf);

    // We cannot inspect text_content from an integration test (pub(crate)),
    // but the goal is: no panic with multi-byte UTF-8.

    teardown();
});

serial_test!(event_format_verification, {
    // Pure format contract test — no FFI calls needed.
    // Verify that manually-built event bytes match the sizes and offsets
    // that the TypeScript EventDecoder expects.

    // --- Keyboard event: [type:u8][key_code:u32le][modifiers:u8][event_type:u8] = 7 bytes ---
    let mut kb = Vec::new();
    let event_keyboard: u8 = 1;
    let key_code: u32 = 65; // 'A'
    let modifiers: u8 = 0b0000_0001; // shift
    let event_type: u8 = 1; // key_down
    kb.push(event_keyboard);
    kb.extend_from_slice(&key_code.to_le_bytes());
    kb.push(modifiers);
    kb.push(event_type);
    assert_eq!(kb.len(), 7, "keyboard event should be 7 bytes");
    assert_eq!(kb[0], 1, "keyboard event type tag");
    assert_eq!(
        u32::from_le_bytes(kb[1..5].try_into().unwrap()),
        65,
        "key_code at offset 1"
    );
    assert_eq!(kb[5], 0b0000_0001, "modifiers at offset 5");
    assert_eq!(kb[6], 1, "event_type at offset 6");

    // --- Mouse event: [type:u8][button:u8][x:u16le][y:u16le][pixel_x:u16le][pixel_y:u16le]
    //                   [modifiers:u8][node_id:u32le] = 15 bytes ---
    let mut mouse = Vec::new();
    let event_mouse: u8 = 2;
    let button: u8 = 0;
    let x: u16 = 10;
    let y: u16 = 20;
    let pixel_x: u16 = 80;
    let pixel_y: u16 = 160;
    let mouse_mods: u8 = 0;
    let node_id: u32 = 42;
    mouse.push(event_mouse);
    mouse.push(button);
    mouse.extend_from_slice(&x.to_le_bytes());
    mouse.extend_from_slice(&y.to_le_bytes());
    mouse.extend_from_slice(&pixel_x.to_le_bytes());
    mouse.extend_from_slice(&pixel_y.to_le_bytes());
    mouse.push(mouse_mods);
    mouse.extend_from_slice(&node_id.to_le_bytes());
    assert_eq!(mouse.len(), 15, "mouse event should be 15 bytes");
    assert_eq!(mouse[0], 2, "mouse event type tag");
    assert_eq!(mouse[1], 0, "button at offset 1");
    assert_eq!(
        u16::from_le_bytes(mouse[2..4].try_into().unwrap()),
        10,
        "x at offset 2"
    );
    assert_eq!(
        u16::from_le_bytes(mouse[4..6].try_into().unwrap()),
        20,
        "y at offset 4"
    );
    assert_eq!(
        u16::from_le_bytes(mouse[6..8].try_into().unwrap()),
        80,
        "pixel_x at offset 6"
    );
    assert_eq!(
        u16::from_le_bytes(mouse[8..10].try_into().unwrap()),
        160,
        "pixel_y at offset 8"
    );
    assert_eq!(mouse[10], 0, "modifiers at offset 10");
    assert_eq!(
        u32::from_le_bytes(mouse[11..15].try_into().unwrap()),
        42,
        "node_id at offset 11"
    );

    // --- Resize event: [type:u8][cols:u16le][rows:u16le][pixel_width:u16le][pixel_height:u16le] = 9 bytes ---
    let mut resize = Vec::new();
    let event_resize: u8 = 3;
    let cols: u16 = 120;
    let rows: u16 = 40;
    let pixel_width: u16 = 960;
    let pixel_height: u16 = 640;
    resize.push(event_resize);
    resize.extend_from_slice(&cols.to_le_bytes());
    resize.extend_from_slice(&rows.to_le_bytes());
    resize.extend_from_slice(&pixel_width.to_le_bytes());
    resize.extend_from_slice(&pixel_height.to_le_bytes());
    assert_eq!(resize.len(), 9, "resize event should be 9 bytes");
    assert_eq!(resize[0], 3, "resize event type tag");
    assert_eq!(
        u16::from_le_bytes(resize[1..3].try_into().unwrap()),
        120,
        "cols at offset 1"
    );
    assert_eq!(
        u16::from_le_bytes(resize[3..5].try_into().unwrap()),
        40,
        "rows at offset 3"
    );
    assert_eq!(
        u16::from_le_bytes(resize[5..7].try_into().unwrap()),
        960,
        "pixel_width at offset 5"
    );
    assert_eq!(
        u16::from_le_bytes(resize[7..9].try_into().unwrap()),
        640,
        "pixel_height at offset 7"
    );
});

serial_test!(init_shutdown_cycle, {
    for _ in 0..10 {
        setup();
        teardown();
    }
    // If we get here without panic/deadlock, the test passes.
});

serial_test!(render_loop_start_stop, {
    setup();

    start_render_loop(30);
    std::thread::sleep(std::time::Duration::from_millis(50));
    stop_render_loop();

    // Small grace period for the background thread to observe the stop flag.
    std::thread::sleep(std::time::Duration::from_millis(50));

    teardown();
    // If we get here without deadlock, the test passes.
});

serial_test!(multiple_render_frames, {
    setup();

    let mut buf = Vec::new();
    buf.extend(encode_create_node(
        1,
        r#"{"width":80,"height":24,"flexDirection":"column"}"#,
    ));
    buf.extend(encode_create_node(2, r#"{"flexGrow":1}"#));
    buf.extend(encode_append_child(1, 2));
    apply_and_render(&buf);

    let first = read_layout(2);

    // Render two more times — layout should be stable.
    render_frame();
    let second = read_layout(2);

    render_frame();
    let third = read_layout(2);

    for i in 0..4 {
        assert_float_eq(first[i], second[i], &format!("render 1 vs 2 index {i}"));
        assert_float_eq(second[i], third[i], &format!("render 2 vs 3 index {i}"));
    }

    teardown();
});
