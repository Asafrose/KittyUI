/**
 * E2E tests for overflow: hidden clipping.
 */

import { describe, test, expect, afterEach } from "bun:test";
import { TestBridge } from "./test-harness/test-bridge.js";
import { VirtualScreen } from "./test-harness/virtual-screen.js";
import "./test-harness/assertions.js";

let bridge: TestBridge;

const canRun = new TestBridge().nativeAvailable;

describe.skipIf(!canRun)("overflow: hidden", () => {
  afterEach(() => {
    bridge?.shutdownTestMode();
  });

  test("red background is clipped to parent width", () => {
    bridge = new TestBridge();
    bridge.initTestMode(40, 10);

    const enc = bridge.getEncoder();

    // Parent: 10 cols wide, overflow hidden
    enc.createNode(1, {
      width: 10,
      height: 1,
      overflow: "hidden",
    });

    // Child: 20 cols wide, red background, no shrink
    enc.createNode(2, {
      width: 20,
      height: 1,
      backgroundColor: "#ff0000",
      flexShrink: 0,
    });
    enc.appendChild(1, 2);
    bridge.flushMutations();
    bridge.renderFrame();

    const output = bridge.getRenderedOutput();
    const screen = new VirtualScreen(40, 10);
    screen.apply(output);

    // Columns 0-9 should have red bg
    for (let col = 0; col < 10; col++) {
      expect(screen).toHaveBgColor(0, col, "#ff0000");
    }

    // Columns 10-19 should NOT have red bg (clipped)
    for (let col = 10; col < 20; col++) {
      const bg = screen.bgAt(0, col);
      expect(bg).not.toBe("#ff0000");
    }
  });
});
