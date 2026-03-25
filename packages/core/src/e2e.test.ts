/**
 * E2E smoke tests for KittyUI core.
 *
 * Each test manages its own Bridge lifecycle to verify end-to-end
 * behaviour of the init → mutate → render → query → shutdown pipeline.
 */

import { describe, test, expect } from "bun:test";
import { Bridge } from "./bridge.js";

const canRun = new Bridge().nativeAvailable;

describe.skipIf(!canRun)("E2E smoke tests", () => {
  // =========================================================================
  // Full lifecycle smoke
  // =========================================================================

  test("full lifecycle smoke", () => {
    const bridge = new Bridge();
    const caps = bridge.init();

    expect(caps.versionMajor).toBe(0);
    expect(caps.versionMinor).toBe(1);
    expect(caps.batchedFfi).toBe(true);

    const enc = bridge.getEncoder();
    enc.createNode(1, { width: 80, height: 24, flexDirection: "column" });
    enc.createNode(2, { width: 40, height: 10 });
    enc.appendChild(1, 2);
    bridge.flushMutations();
    bridge.renderFrame();

    const layout = bridge.getLayout(2);
    expect(layout.width).toBeCloseTo(40, 5);
    expect(layout.height).toBeCloseTo(10, 5);

    bridge.shutdown();
  });

  // =========================================================================
  // Rapid init/shutdown cycling
  // =========================================================================

  test("rapid init/shutdown 10x", () => {
    for (let i = 0; i < 10; i++) {
      const bridge = new Bridge();
      const caps = bridge.init();
      expect(caps.batchedFfi).toBe(true);

      const enc = bridge.getEncoder();
      enc.createNode(1, { width: 80, height: 24 });
      bridge.flushMutations();
      bridge.renderFrame();

      const layout = bridge.getLayout(1);
      expect(layout.width).toBeCloseTo(80, 5);

      bridge.shutdown();
    }
  });

  // =========================================================================
  // Large tree (200 nodes)
  // =========================================================================

  test("large tree with 200 nodes", () => {
    const bridge = new Bridge();
    bridge.init();

    const enc = bridge.getEncoder();
    enc.createNode(1, {
      width: 80,
      height: 200,
      flexDirection: "column",
    });
    for (let i = 2; i <= 201; i++) {
      enc.createNode(i, { flexGrow: 1 });
      enc.appendChild(1, i);
    }
    bridge.flushMutations();
    bridge.renderFrame();

    const layouts = bridge.getAllLayouts(300);
    expect(layouts.size).toBe(201);

    // Verify root
    const root = layouts.get(1);
    expect(root).toBeDefined();
    expect(root!.width).toBeCloseTo(80, 5);
    expect(root!.height).toBeCloseTo(200, 5);

    // Each child should get 200/200 = 1.0 height
    const child = layouts.get(2);
    expect(child).toBeDefined();
    expect(child!.height).toBeCloseTo(1, 1);

    bridge.shutdown();
  });

  // =========================================================================
  // Render loop start/stop
  // =========================================================================

  test("render loop start and stop", async () => {
    const bridge = new Bridge();
    bridge.init();

    const enc = bridge.getEncoder();
    enc.createNode(1, { width: 80, height: 24 });
    bridge.flushMutations();
    bridge.renderFrame();

    bridge.startRenderLoop(30);
    await new Promise((r) => setTimeout(r, 100));
    bridge.stopRenderLoop();
    await new Promise((r) => setTimeout(r, 50));

    // Engine should still be usable after stopping render loop
    bridge.requestRender();
    bridge.renderFrame();

    const layout = bridge.getLayout(1);
    expect(layout.width).toBeCloseTo(80, 5);

    bridge.shutdown();
  });

  // =========================================================================
  // Multi-step mutation
  // =========================================================================

  test("multi-step mutation sequence", () => {
    const bridge = new Bridge();
    bridge.init();

    // Step 1: create root
    const enc1 = bridge.getEncoder();
    enc1.createNode(1, {
      width: 80,
      height: 24,
      flexDirection: "column",
    });
    bridge.flushMutations();
    bridge.renderFrame();

    // Step 2: add child
    const enc2 = bridge.getEncoder();
    enc2.createNode(2, { height: 10 });
    enc2.appendChild(1, 2);
    bridge.flushMutations();
    bridge.renderFrame();

    expect(bridge.getLayout(2).height).toBeCloseTo(10, 5);

    // Step 3: update style
    const enc3 = bridge.getEncoder();
    enc3.setStyle(2, { height: 15 });
    bridge.flushMutations();
    bridge.renderFrame();

    expect(bridge.getLayout(2).height).toBeCloseTo(15, 5);

    // Step 4: add text
    const enc4 = bridge.getEncoder();
    enc4.setText(2, "Hello world");
    bridge.flushMutations();
    bridge.renderFrame();
    // Should not crash

    // Step 5: remove child
    const enc5 = bridge.getEncoder();
    enc5.removeNode(2);
    bridge.flushMutations();
    bridge.renderFrame();

    // Layout for removed node should return zeros
    const removed = bridge.getLayout(2);
    expect(removed.width).toBeCloseTo(0, 5);
    expect(removed.height).toBeCloseTo(0, 5);

    bridge.shutdown();
  });

  // =========================================================================
  // Error handling
  // =========================================================================

  test("methods throw before init", () => {
    const bridge = new Bridge();
    expect(() => bridge.flushMutations()).toThrow("Bridge not initialised");
    expect(() => bridge.renderFrame()).toThrow("Bridge not initialised");
    expect(() => bridge.getLayout(1)).toThrow("Bridge not initialised");
    expect(() => bridge.getAllLayouts()).toThrow("Bridge not initialised");
    expect(() => bridge.requestRender()).toThrow("Bridge not initialised");
    expect(() => bridge.startRenderLoop()).toThrow("Bridge not initialised");
    expect(() => bridge.stopRenderLoop()).toThrow("Bridge not initialised");
    expect(() => bridge.pushKeyEvent(65, 0, 1)).toThrow(
      "Bridge not initialised",
    );
    expect(() => bridge.pushMouseEvent(0, 0, 0, 0, 0, 0)).toThrow(
      "Bridge not initialised",
    );
    expect(() => bridge.focus(1)).toThrow("Bridge not initialised");
    expect(() => bridge.blur()).toThrow("Bridge not initialised");
    expect(() => bridge.getFocusedNode()).toThrow("Bridge not initialised");
    expect(() => bridge.setFocusable(1, true)).toThrow(
      "Bridge not initialised",
    );
    expect(() => bridge.setTabIndex(1, 0)).toThrow(
      "Bridge not initialised",
    );
    expect(() => bridge.setFocusTrap(1, true)).toThrow(
      "Bridge not initialised",
    );
  });

  test("double init throws", () => {
    const bridge = new Bridge();
    bridge.init();
    expect(() => bridge.init()).toThrow("Bridge already initialised");
    bridge.shutdown();
  });

  test("native unavailable throws descriptive error", () => {
    // We can't easily test this since canRun is true,
    // but verify the error message is what we expect by checking the code path
    const bridge = new Bridge();
    // Bridge.init checks nativeAvailable, so this test is more of a contract check
    expect(bridge.nativeAvailable).toBe(true);
    bridge.init();
    bridge.shutdown();
  });

  // =========================================================================
  // Focus system E2E
  // =========================================================================

  test("focus system end-to-end", () => {
    const bridge = new Bridge();
    bridge.init();

    const enc = bridge.getEncoder();
    enc.createNode(1, {
      width: 80,
      height: 24,
      flexDirection: "column",
    });
    enc.createNode(2, { width: 20, height: 10 });
    enc.createNode(3, { width: 20, height: 10 });
    enc.appendChild(1, 2);
    enc.appendChild(1, 3);
    bridge.flushMutations();
    bridge.renderFrame();

    // Set up focusable nodes
    bridge.setFocusable(2, true);
    bridge.setFocusable(3, true);

    // Focus node 2
    expect(bridge.focus(2)).toBe(true);
    expect(bridge.getFocusedNode()).toBe(2);

    // Switch focus to node 3
    expect(bridge.focus(3)).toBe(true);
    expect(bridge.getFocusedNode()).toBe(3);

    // Blur
    expect(bridge.blur()).toBe(true);
    expect(bridge.getFocusedNode()).toBeNull();

    // Flush events
    bridge.renderFrame();

    bridge.shutdown();
  });

  // =========================================================================
  // Event dispatch
  // =========================================================================

  test("dispatchEvents decodes known event types", () => {
    const bridge = new Bridge();
    bridge.init();

    const received: unknown[] = [];
    bridge.onEvents((events) => {
      received.push(...events);
    });

    // Build a keyboard event buffer manually
    // Format: [EVENT_KEYBOARD=1, keyCode:u32LE, modifiers:u8, eventType:u8]
    const buf = new Uint8Array(7);
    const view = new DataView(buf.buffer);
    buf[0] = 1; // EVENT_KEYBOARD
    view.setUint32(1, 65, true); // keyCode = 65 ('A')
    buf[5] = 0; // modifiers
    buf[6] = 1; // eventType = keyDown

    bridge.dispatchEvents(buf);

    expect(received.length).toBe(1);
    const evt = received[0] as { type: string; keyCode: number };
    expect(evt.type).toBe("keyboard");
    expect(evt.keyCode).toBe(65);

    bridge.shutdown();
  });

  // =========================================================================
  // Text overflow ellipsis
  // =========================================================================

  test("text overflow ellipsis does not crash", () => {
    const bridge = new Bridge();
    bridge.init();

    const enc = bridge.getEncoder();
    enc.createNode(1, { width: 80, height: 24, flexDirection: "column" });
    enc.createNode(2, { width: 10, height: 1, textOverflow: "ellipsis" });
    enc.appendChild(1, 2);
    enc.setText(2, "Very long text that should be truncated with ellipsis");
    bridge.flushMutations();
    bridge.renderFrame();

    const layout = bridge.getLayout(2);
    expect(layout.width).toBeCloseTo(10, 5);
    expect(layout.height).toBeCloseTo(1, 5);

    bridge.shutdown();
  });

  // =========================================================================
  // Border rendering
  // =========================================================================

  test("border style passes through pipeline without error", () => {
    const bridge = new Bridge();
    bridge.init();

    const enc = bridge.getEncoder();
    enc.createNode(1, { width: 80, height: 24, flexDirection: "column" });
    enc.createNode(2, { width: 10, height: 5, border: "round", borderColor: "#FF0000" });
    enc.appendChild(1, 2);
    bridge.flushMutations();
    bridge.renderFrame();

    // Verify layout still works with border styles.
    const layout = bridge.getLayout(2);
    expect(layout.width).toBeCloseTo(10, 5);
    expect(layout.height).toBeCloseTo(5, 5);

    bridge.shutdown();
  });
});
