/**
 * Bridge — TypeScript FFI contract tests.
 *
 * These tests verify the Bridge class correctly communicates with the Rust
 * engine through the batched FFI protocol.
 *
 * Tests are skipped when the native library is not built.
 */

import { describe, test, expect, beforeEach, afterEach } from "bun:test";
import { Bridge, type InitResult, type NodeLayout } from "./bridge.js";
import { MutationEncoder } from "./mutation-encoder.js";

// ---------------------------------------------------------------------------
// Skip guard — tests require the native .dylib / .so to be built
// ---------------------------------------------------------------------------

const canRun = new Bridge().nativeAvailable;

describe.skipIf(!canRun)("Bridge FFI contract", () => {
  let bridge: Bridge;

  beforeEach(() => {
    bridge = new Bridge();
    bridge.init();
  });

  afterEach(() => {
    bridge.shutdown();
  });

  // =========================================================================
  // Lifecycle
  // =========================================================================

  describe("lifecycle", () => {
    test("init returns capabilities", () => {
      // We already called init in beforeEach — create a fresh bridge
      bridge.shutdown();
      const b2 = new Bridge();
      const caps: InitResult = b2.init();
      expect(caps.versionMajor).toBe(0);
      expect(caps.versionMinor).toBe(1);
      expect(caps.versionPatch).toBe(0);
      expect(caps.batchedFfi).toBe(true);
      b2.shutdown();
    });

    test("double-init throws", () => {
      // bridge is already initialised in beforeEach
      expect(() => bridge.init()).toThrow("Bridge already initialised");
    });

    test("shutdown is idempotent", () => {
      bridge.shutdown();
      // Second shutdown should not throw
      bridge.shutdown();
    });

    test("methods throw before init", () => {
      const uninit = new Bridge();
      expect(() => uninit.flushMutations()).toThrow(
        "Bridge not initialised",
      );
      expect(() => uninit.renderFrame()).toThrow("Bridge not initialised");
      expect(() => uninit.getLayout(1)).toThrow("Bridge not initialised");
      expect(() => uninit.requestRender()).toThrow("Bridge not initialised");
    });
  });

  // =========================================================================
  // Layout — single node
  // =========================================================================

  describe("layout", () => {
    test("single node has correct dimensions", () => {
      const enc = bridge.getEncoder();
      enc.createNode(1, { width: 40, height: 12 });
      bridge.flushMutations();
      bridge.renderFrame();

      const layout = bridge.getLayout(1);
      expect(layout.x).toBeCloseTo(0, 5);
      expect(layout.y).toBeCloseTo(0, 5);
      expect(layout.width).toBeCloseTo(40, 5);
      expect(layout.height).toBeCloseTo(12, 5);
    });

    test("column layout stacks children vertically", () => {
      const enc = bridge.getEncoder();
      enc.createNode(1, {
        width: 80,
        height: 24,
        flexDirection: "column",
      });
      enc.createNode(2, { height: 6 });
      enc.createNode(3, { height: 8 });
      enc.appendChild(1, 2);
      enc.appendChild(1, 3);
      bridge.flushMutations();
      bridge.renderFrame();

      const l2 = bridge.getLayout(2);
      const l3 = bridge.getLayout(3);
      expect(l2.y).toBeCloseTo(0, 5);
      expect(l3.y).toBeCloseTo(6, 5);
      expect(l2.height).toBeCloseTo(6, 5);
      expect(l3.height).toBeCloseTo(8, 5);
    });

    test("row layout stacks children horizontally", () => {
      const enc = bridge.getEncoder();
      enc.createNode(1, { width: 80, height: 24 });
      enc.createNode(2, { width: 20, height: 10 });
      enc.createNode(3, { width: 30, height: 10 });
      enc.appendChild(1, 2);
      enc.appendChild(1, 3);
      bridge.flushMutations();
      bridge.renderFrame();

      const l2 = bridge.getLayout(2);
      const l3 = bridge.getLayout(3);
      expect(l2.x).toBeCloseTo(0, 5);
      expect(l3.x).toBeCloseTo(20, 5);
    });

    test("flexGrow distributes space equally", () => {
      const enc = bridge.getEncoder();
      enc.createNode(1, {
        width: 80,
        height: 24,
        flexDirection: "column",
      });
      enc.createNode(2, { flexGrow: 1 });
      enc.createNode(3, { flexGrow: 1 });
      enc.appendChild(1, 2);
      enc.appendChild(1, 3);
      bridge.flushMutations();
      bridge.renderFrame();

      const l2 = bridge.getLayout(2);
      const l3 = bridge.getLayout(3);
      expect(l2.height).toBeCloseTo(12, 1);
      expect(l3.height).toBeCloseTo(12, 1);
      expect(l3.y).toBeCloseTo(12, 1);
    });

    test("padding reduces content area", () => {
      const enc = bridge.getEncoder();
      enc.createNode(1, {
        width: 80,
        height: 24,
        flexDirection: "column",
        padding: 2,
      });
      enc.createNode(2, { flexGrow: 1 });
      enc.appendChild(1, 2);
      bridge.flushMutations();
      bridge.renderFrame();

      const l2 = bridge.getLayout(2);
      expect(l2.x).toBeCloseTo(2, 1);
      expect(l2.y).toBeCloseTo(2, 1);
      expect(l2.width).toBeCloseTo(76, 1);
      expect(l2.height).toBeCloseTo(20, 1);
    });

    test("setStyle updates node", () => {
      const enc = bridge.getEncoder();
      enc.createNode(1, { width: 80, height: 24 });
      enc.createNode(2, { width: 10, height: 5 });
      enc.appendChild(1, 2);
      bridge.flushMutations();
      bridge.renderFrame();

      expect(bridge.getLayout(2).width).toBeCloseTo(10, 5);

      const enc2 = bridge.getEncoder();
      enc2.setStyle(2, { width: 25, height: 5 });
      bridge.flushMutations();
      bridge.renderFrame();

      expect(bridge.getLayout(2).width).toBeCloseTo(25, 5);
    });

    test("removeNode cleans up", () => {
      const enc = bridge.getEncoder();
      enc.createNode(1, {
        width: 80,
        height: 24,
        flexDirection: "column",
      });
      enc.createNode(2, { height: 10 });
      enc.appendChild(1, 2);
      bridge.flushMutations();
      bridge.renderFrame();

      const enc2 = bridge.getEncoder();
      enc2.removeNode(2);
      bridge.flushMutations();

      // Layout for removed node should return zeros
      const layout = bridge.getLayout(2);
      expect(layout.x).toBeCloseTo(0, 5);
      expect(layout.y).toBeCloseTo(0, 5);
      expect(layout.width).toBeCloseTo(0, 5);
      expect(layout.height).toBeCloseTo(0, 5);
    });

    test("getAllLayouts returns all nodes", () => {
      const enc = bridge.getEncoder();
      enc.createNode(1, {
        width: 80,
        height: 24,
        flexDirection: "column",
      });
      for (let i = 2; i <= 6; i++) {
        enc.createNode(i, { height: 4 });
        enc.appendChild(1, i);
      }
      bridge.flushMutations();
      bridge.renderFrame();

      const layouts = bridge.getAllLayouts();
      expect(layouts.size).toBe(6); // 1 root + 5 children
    });

    test("multiple mutation batches", () => {
      // First batch
      const enc1 = bridge.getEncoder();
      enc1.createNode(1, {
        width: 80,
        height: 24,
        flexDirection: "column",
      });
      bridge.flushMutations();

      // Second batch
      const enc2 = bridge.getEncoder();
      enc2.createNode(2, { height: 10 });
      enc2.appendChild(1, 2);
      bridge.flushMutations();
      bridge.renderFrame();

      const layout = bridge.getLayout(2);
      expect(layout.height).toBeCloseTo(10, 5);
    });
  });

  // =========================================================================
  // Stress test
  // =========================================================================

  describe("stress", () => {
    test("100 nodes layout correctly", () => {
      const enc = bridge.getEncoder();
      enc.createNode(1, {
        width: 80,
        height: 100,
        flexDirection: "column",
      });
      for (let i = 2; i <= 101; i++) {
        enc.createNode(i, { flexGrow: 1 });
        enc.appendChild(1, i);
      }
      bridge.flushMutations();
      bridge.renderFrame();

      const layouts = bridge.getAllLayouts(200);
      expect(layouts.size).toBe(101); // root + 100 children

      // Verify root dimensions
      const root = layouts.get(1);
      expect(root).toBeDefined();
      expect(root!.width).toBeCloseTo(80, 5);
      expect(root!.height).toBeCloseTo(100, 5);
    });
  });

  // =========================================================================
  // Focus system
  // =========================================================================

  describe("focus", () => {
    test("focus and blur lifecycle", () => {
      const enc = bridge.getEncoder();
      enc.createNode(1, { width: 80, height: 24 });
      bridge.flushMutations();

      bridge.setFocusable(1, true);
      const focused = bridge.focus(1);
      expect(focused).toBe(true);
      expect(bridge.getFocusedNode()).toBe(1);

      const blurred = bridge.blur();
      expect(blurred).toBe(true);
      expect(bridge.getFocusedNode()).toBeNull();
    });

    test("focus non-focusable node fails", () => {
      const enc = bridge.getEncoder();
      enc.createNode(1, { width: 80, height: 24 });
      bridge.flushMutations();

      const result = bridge.focus(1);
      expect(result).toBe(false);
    });

    test("focus nonexistent node fails", () => {
      const result = bridge.focus(999);
      expect(result).toBe(false);
    });

    test("blur with nothing focused returns false", () => {
      const result = bridge.blur();
      expect(result).toBe(false);
    });

    test("setTabIndex works", () => {
      const enc = bridge.getEncoder();
      enc.createNode(1, {
        width: 80,
        height: 24,
        flexDirection: "column",
      });
      enc.createNode(2, { width: 20, height: 5 });
      enc.createNode(3, { width: 20, height: 5 });
      enc.appendChild(1, 2);
      enc.appendChild(1, 3);
      bridge.flushMutations();

      bridge.setTabIndex(2, 0);
      bridge.setTabIndex(3, 1);

      const result = bridge.focus(3);
      expect(result).toBe(true);
      expect(bridge.getFocusedNode()).toBe(3);
    });

    test("setFocusTrap does not crash", () => {
      const enc = bridge.getEncoder();
      enc.createNode(1, { width: 80, height: 24 });
      bridge.flushMutations();

      bridge.setFocusTrap(1, true);
      bridge.setFocusTrap(1, false);
      // Should not throw
    });
  });

  // =========================================================================
  // Input
  // =========================================================================

  describe("input", () => {
    test("pushKeyEvent does not crash", () => {
      bridge.pushKeyEvent(65, 0, 1);
      bridge.renderFrame();
    });

    test("pushMouseEvent does not crash", () => {
      const enc = bridge.getEncoder();
      enc.createNode(1, { width: 80, height: 24 });
      bridge.flushMutations();
      bridge.renderFrame();

      bridge.pushMouseEvent(0, 10, 10, 80, 80, 0);
      bridge.renderFrame();
    });
  });

  // =========================================================================
  // Render loop
  // =========================================================================

  describe("render loop", () => {
    test("start and stop render loop", async () => {
      bridge.startRenderLoop(30);
      await new Promise((r) => setTimeout(r, 100));
      bridge.stopRenderLoop();
      await new Promise((r) => setTimeout(r, 50));
      // Should not throw
    });
  });

  // =========================================================================
  // Encoder integration
  // =========================================================================

  describe("encoder", () => {
    test("empty flush is a no-op", () => {
      // Encoder has 0 bytes — flushMutations should silently skip
      bridge.flushMutations();
      bridge.renderFrame();
    });

    test("encoder reset after flush", () => {
      const enc = bridge.getEncoder();
      enc.createNode(1, { width: 80, height: 24 });
      expect(enc.byteLength).toBeGreaterThan(0);

      bridge.flushMutations();
      expect(enc.byteLength).toBe(0);
    });
  });
});
