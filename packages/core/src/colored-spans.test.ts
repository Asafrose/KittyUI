/**
 * E2E tests for inline colored spans (issue #102).
 *
 * Verifies that text color spans are correctly painted per-character.
 */

import { describe, test, expect, afterEach } from "bun:test";
import { TestBridge } from "./test-harness/test-bridge.js";
import { VirtualScreen } from "./test-harness/virtual-screen.js";

let bridge: TestBridge;

const canRun = new TestBridge().nativeAvailable;

describe.skipIf(!canRun)("colored spans", () => {
  afterEach(() => {
    bridge?.shutdownTestMode();
  });

  test("two colored spans render side by side on the same row", () => {
    bridge = new TestBridge();
    bridge.initTestMode(80, 24);

    const enc = bridge.getEncoder();

    // Root container
    enc.createNode(1, {
      width: 80,
      height: 24,
    });

    // Text node with 2-char content
    enc.createNode(2, {
      width: 2,
      height: 1,
    });
    enc.setText(2, "AB");
    enc.setTextSpans(2, [
      { start: 0, end: 1, r: 255, g: 0, b: 0 },   // A = red
      { start: 1, end: 2, r: 0, g: 0, b: 255 },    // B = blue
    ]);
    enc.appendChild(1, 2);

    bridge.flushMutations();
    bridge.renderFrame();

    const output = bridge.getRenderedOutput();
    const screen = new VirtualScreen(80, 24);
    screen.apply(output);

    expect(screen.textAt(0, 0)).toBe("A");
    expect(screen.fgAt(0, 0)).toBe("#ff0000");
    expect(screen.textAt(0, 1)).toBe("B");
    expect(screen.fgAt(0, 1)).toBe("#0000ff");
  });

  test("three spans with different colors all visible", () => {
    bridge = new TestBridge();
    bridge.initTestMode(80, 24);

    const enc = bridge.getEncoder();

    enc.createNode(1, {
      width: 80,
      height: 24,
    });

    enc.createNode(2, {
      width: 9,
      height: 1,
    });
    enc.setText(2, "RedGrnBlu");
    enc.setTextSpans(2, [
      { start: 0, end: 3, r: 255, g: 0, b: 0 },
      { start: 3, end: 6, r: 0, g: 255, b: 0 },
      { start: 6, end: 9, r: 0, g: 0, b: 255 },
    ]);
    enc.appendChild(1, 2);

    bridge.flushMutations();
    bridge.renderFrame();

    const output = bridge.getRenderedOutput();
    const screen = new VirtualScreen(80, 24);
    screen.apply(output);

    expect(screen.fgAt(0, 0)).toBe("#ff0000");
    expect(screen.fgAt(0, 3)).toBe("#00ff00");
    expect(screen.fgAt(0, 6)).toBe("#0000ff");
  });

  test("spans can be updated after initial render", () => {
    bridge = new TestBridge();
    bridge.initTestMode(80, 24);

    const enc = bridge.getEncoder();

    enc.createNode(1, {
      width: 80,
      height: 24,
    });

    enc.createNode(2, {
      width: 2,
      height: 1,
    });
    enc.setText(2, "XY");
    enc.setTextSpans(2, [
      { start: 0, end: 1, r: 255, g: 0, b: 0 },
      { start: 1, end: 2, r: 0, g: 255, b: 0 },
    ]);
    enc.appendChild(1, 2);

    bridge.flushMutations();
    bridge.renderFrame();

    let output = bridge.getRenderedOutput();
    let screen = new VirtualScreen(80, 24);
    screen.apply(output);

    expect(screen.fgAt(0, 0)).toBe("#ff0000");
    expect(screen.fgAt(0, 1)).toBe("#00ff00");

    // Update: swap colors
    enc.setTextSpans(2, [
      { start: 0, end: 1, r: 0, g: 255, b: 0 },
      { start: 1, end: 2, r: 255, g: 0, b: 0 },
    ]);

    bridge.flushMutations();
    bridge.renderFrame();

    output = bridge.getRenderedOutput();
    screen = new VirtualScreen(80, 24);
    screen.apply(output);

    expect(screen.fgAt(0, 0)).toBe("#00ff00");
    expect(screen.fgAt(0, 1)).toBe("#ff0000");
  });

  test("text without spans uses node fg color", () => {
    bridge = new TestBridge();
    bridge.initTestMode(80, 24);

    const enc = bridge.getEncoder();

    enc.createNode(1, {
      width: 80,
      height: 24,
    });

    enc.createNode(2, {
      width: 5,
      height: 1,
      color: "#808080",
    });
    enc.setText(2, "Hello");
    enc.appendChild(1, 2);

    bridge.flushMutations();
    bridge.renderFrame();

    const output = bridge.getRenderedOutput();
    const screen = new VirtualScreen(80, 24);
    screen.apply(output);

    expect(screen.fgAt(0, 0)).toBe("#808080");
  });
});
