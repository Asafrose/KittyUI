/**
 * E2E Bridge/FFI integration tests for KittyUI.
 *
 * Tests TestBridge lifecycle, mutation encoding round-trips,
 * layout queries, large batches, rapid render cycles, and cleanup.
 */

import { describe, test, expect, afterEach } from "bun:test";
import React from "react";

declare module "react" {
  // eslint-disable-next-line @typescript-eslint/no-namespace
  namespace JSX {
    interface IntrinsicElements {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      [elemName: string]: any;
    }
  }
}

import { TestBridge } from "@kittyui/core/src/test-harness/test-bridge.js";
import { VirtualScreen } from "@kittyui/core/src/test-harness/virtual-screen.js";
import { MutationEncoder, RenderableTree, resetNodeIdCounter } from "@kittyui/core";
import "@kittyui/core/src/test-harness/assertions.js";
import { render, type RenderResult } from "../test-utils/render-jsx.js";

const canRun = new TestBridge().nativeAvailable;

describe.skipIf(!canRun)("E2E Bridge", () => {
  let result: RenderResult | undefined;

  afterEach(() => {
    result?.cleanup();
    result = undefined;
  });

  // ==========================================================================
  // TestBridge lifecycle
  // ==========================================================================

  describe("TestBridge lifecycle", () => {
    test("initTestMode returns valid result", () => {
      const bridge = new TestBridge();
      const initResult = bridge.initTestMode(40, 10);
      expect(initResult.versionMajor).toBeGreaterThanOrEqual(0);
      expect(initResult.versionMinor).toBeGreaterThanOrEqual(0);
      expect(initResult.versionPatch).toBeGreaterThanOrEqual(0);
      bridge.shutdownTestMode();
    });

    test("shutdown after init does not throw", () => {
      const bridge = new TestBridge();
      bridge.initTestMode(20, 10);
      expect(() => bridge.shutdownTestMode()).not.toThrow();
    });

    test("double shutdown is safe", () => {
      const bridge = new TestBridge();
      bridge.initTestMode(20, 10);
      bridge.shutdownTestMode();
      expect(() => bridge.shutdownTestMode()).not.toThrow();
    });

    test("nativeAvailable returns true when lib exists", () => {
      const bridge = new TestBridge();
      expect(bridge.nativeAvailable).toBe(true);
    });
  });

  // ==========================================================================
  // Layout queries
  // ==========================================================================

  describe("layout queries", () => {
    test("getLayout returns valid layout for root", async () => {
      result = await render(
        <box style={{ width: 30, height: 8 }}><text>L</text></box>,
        { cols: 30, rows: 8 },
      );
      const layout = result.getLayout(1);
      expect(layout.width).toBeGreaterThan(0);
      expect(layout.height).toBeGreaterThan(0);
    });

    test("getAllLayouts returns multiple nodes", async () => {
      result = await render(
        <box style={{ width: 20, height: 5 }}>
          <box style={{ width: 10, height: 2 }}><text>A</text></box>
          <box style={{ width: 10, height: 2 }}><text>B</text></box>
        </box>,
        { cols: 20, rows: 5 },
      );
      const layouts = result.getAllLayouts();
      expect(layouts.size).toBeGreaterThanOrEqual(3);
    });

    test("layout x,y are non-negative", async () => {
      result = await render(
        <box style={{ width: 20, height: 5 }}>
          <box style={{ width: 10, height: 3 }}><text>P</text></box>
        </box>,
        { cols: 20, rows: 5 },
      );
      const layouts = result.getAllLayouts();
      for (const [, layout] of layouts) {
        expect(layout.x).toBeGreaterThanOrEqual(0);
        expect(layout.y).toBeGreaterThanOrEqual(0);
      }
    });

    test("layout dimensions match style", async () => {
      result = await render(
        <box style={{ width: 15, height: 7 }}><text>S</text></box>,
        { cols: 40, rows: 10 },
      );
      const layouts = result.getAllLayouts();
      let found = false;
      for (const [, l] of layouts) {
        if (Math.abs(l.width - 15) < 1 && Math.abs(l.height - 7) < 1) {
          found = true;
        }
      }
      expect(found).toBe(true);
    });
  });

  // ==========================================================================
  // Rendered output
  // ==========================================================================

  describe("rendered output", () => {
    test("render produces non-empty output with content", async () => {
      result = await render(
        <box style={{ width: 10, height: 3 }}><text>Test</text></box>,
        { cols: 10, rows: 3 },
      );
      expect(result.screen).toContainText("Test");
    });

    test("render empty box produces output", async () => {
      result = await render(
        <box style={{ backgroundColor: "#ff0000", width: 5, height: 3 }} />,
        { cols: 10, rows: 5 },
      );
      expect(result.screen).toHaveBgColor(0, 0, "#ff0000");
    });

    test("multiple render cycles produce consistent output", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}><text>Stable</text></box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("Stable");

      const screen2 = await result.rerender(
        <box style={{ width: 20, height: 3 }}><text>Stable</text></box>,
      );
      expect(screen2).toContainText("Stable");
    });
  });

  // ==========================================================================
  // Large mutation batches
  // ==========================================================================

  describe("large mutation batches", () => {
    test("render 50 children", async () => {
      const children = Array.from({ length: 50 }, (_, i) =>
        <box key={i} style={{ height: 1 }}><text>{`N${i}`}</text></box>
      );
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 50 }}>
          {children}
        </box>,
        { cols: 20, rows: 50 },
      );
      expect(result.screen).toContainText("N0");
      expect(result.screen).toContainText("N49");
    });

    test("render 100 children", async () => {
      const children = Array.from({ length: 100 }, (_, i) =>
        <box key={i} style={{ height: 1 }}><text>{`X${i}`}</text></box>
      );
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 100 }}>
          {children}
        </box>,
        { cols: 20, rows: 100 },
      );
      expect(result.screen).toContainText("X0");
      expect(result.screen).toContainText("X99");
    });
  });

  // ==========================================================================
  // Rapid render cycles
  // ==========================================================================

  describe("rapid render cycles", () => {
    test("10 rapid rerenders", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}><text>V0</text></box>,
        { cols: 20, rows: 3 },
      );
      for (let i = 1; i <= 10; i++) {
        await result.rerender(
          <box style={{ width: 20, height: 3 }}><text>{`V${i}`}</text></box>,
        );
      }
      const finalScreen = await result.rerender(
        <box style={{ width: 20, height: 3 }}><text>V10</text></box>,
      );
      expect(finalScreen).toContainText("V10");
    });

    test("20 rapid rerenders with changing styles", async () => {
      result = await render(
        <box style={{ width: 10, height: 3 }}><text>S0</text></box>,
        { cols: 40, rows: 10 },
      );
      for (let i = 1; i <= 20; i++) {
        await result.rerender(
          <box style={{ width: 10 + i, height: 3 }}><text>{`S${i}`}</text></box>,
        );
      }
      const finalScreen = await result.rerender(
        <box style={{ width: 30, height: 3 }}><text>S20</text></box>,
      );
      expect(finalScreen).toContainText("S20");
    });
  });

  // ==========================================================================
  // VirtualScreen operations
  // ==========================================================================

  describe("VirtualScreen direct", () => {
    test("empty VirtualScreen has only spaces", () => {
      const screen = new VirtualScreen(10, 5);
      expect(screen.textAt(0, 0)).toBe(" ");
      expect(screen.textAt(4, 9)).toBe(" ");
    });

    test("empty screen containsText returns false", () => {
      const screen = new VirtualScreen(10, 5);
      expect(screen.containsText("hello")).toBe(false);
    });

    test("screen dimensions match constructor", () => {
      const screen = new VirtualScreen(20, 8);
      expect(screen.cols).toBe(20);
      expect(screen.rows).toBe(8);
    });

    test("cellAt out of bounds returns undefined", () => {
      const screen = new VirtualScreen(10, 5);
      expect(screen.cellAt(-1, 0)).toBeUndefined();
      expect(screen.cellAt(0, -1)).toBeUndefined();
      expect(screen.cellAt(5, 0)).toBeUndefined();
      expect(screen.cellAt(0, 10)).toBeUndefined();
    });

    test("getRowText on empty screen returns spaces", () => {
      const screen = new VirtualScreen(5, 3);
      expect(screen.getRowText(0)).toBe("     ");
    });

    test("getRowText out of bounds returns empty string", () => {
      const screen = new VirtualScreen(5, 3);
      expect(screen.getRowText(-1)).toBe("");
      expect(screen.getRowText(3)).toBe("");
    });

    test("toString returns full screen content", () => {
      const screen = new VirtualScreen(3, 2);
      const str = screen.toString();
      expect(str).toContain("   ");
    });

    test("findText on empty screen returns undefined", () => {
      const screen = new VirtualScreen(10, 5);
      expect(screen.findText("hello")).toBeUndefined();
    });
  });

  // ==========================================================================
  // ANSI parsing via VirtualScreen
  // ==========================================================================

  describe("ANSI parsing", () => {
    test("parse cursor position and character", () => {
      const screen = new VirtualScreen(10, 5);
      // ESC[2;3H positions cursor at row 2, col 3 (1-based) → row 1, col 2 (0-based)
      // Then write 'A'
      const data = new Uint8Array([0x1b, 0x5b, 0x32, 0x3b, 0x33, 0x48, 0x41]);
      screen.apply(data);
      expect(screen.textAt(1, 2)).toBe("A");
    });

    test("parse SGR reset", () => {
      const screen = new VirtualScreen(10, 5);
      // ESC[0m (reset) then 'X'
      const data = new Uint8Array([0x1b, 0x5b, 0x30, 0x6d, 0x58]);
      screen.apply(data);
      expect(screen.textAt(0, 0)).toBe("X");
      expect(screen.fgAt(0, 0)).toBeUndefined();
      expect(screen.bgAt(0, 0)).toBeUndefined();
    });

    test("parse SGR bold", () => {
      const screen = new VirtualScreen(10, 5);
      // ESC[1m (bold) then 'B'
      const data = new Uint8Array([0x1b, 0x5b, 0x31, 0x6d, 0x42]);
      screen.apply(data);
      expect(screen.textAt(0, 0)).toBe("B");
      expect(screen.cellAt(0, 0)?.bold).toBe(true);
    });

    test("parse SGR italic", () => {
      const screen = new VirtualScreen(10, 5);
      // ESC[3m (italic) then 'I'
      const data = new Uint8Array([0x1b, 0x5b, 0x33, 0x6d, 0x49]);
      screen.apply(data);
      expect(screen.textAt(0, 0)).toBe("I");
      expect(screen.cellAt(0, 0)?.italic).toBe(true);
    });

    test("parse SGR fg RGB", () => {
      const screen = new VirtualScreen(10, 5);
      // ESC[38;2;255;0;0m (fg red) then 'R'
      const ansi = "\x1b[38;2;255;0;0mR";
      const data = new Uint8Array(Buffer.from(ansi));
      screen.apply(data);
      expect(screen.textAt(0, 0)).toBe("R");
      expect(screen.fgAt(0, 0)).toBe("#ff0000");
    });

    test("parse SGR bg RGB", () => {
      const screen = new VirtualScreen(10, 5);
      // ESC[48;2;0;255;0m (bg green) then 'G'
      const ansi = "\x1b[48;2;0;255;0mG";
      const data = new Uint8Array(Buffer.from(ansi));
      screen.apply(data);
      expect(screen.textAt(0, 0)).toBe("G");
      expect(screen.bgAt(0, 0)).toBe("#00ff00");
    });

    test("parse multiple characters at different positions", () => {
      const screen = new VirtualScreen(10, 5);
      // ESC[1;1HA ESC[2;5HB
      const ansi = "\x1b[1;1HA\x1b[2;5HB";
      const data = new Uint8Array(Buffer.from(ansi));
      screen.apply(data);
      expect(screen.textAt(0, 0)).toBe("A");
      expect(screen.textAt(1, 4)).toBe("B");
    });

    test("parse combined bold + fg color", () => {
      const screen = new VirtualScreen(10, 5);
      // ESC[1;38;2;0;0;255m (bold + fg blue) then 'X'
      const ansi = "\x1b[1;38;2;0;0;255mX";
      const data = new Uint8Array(Buffer.from(ansi));
      screen.apply(data);
      expect(screen.textAt(0, 0)).toBe("X");
      expect(screen.cellAt(0, 0)?.bold).toBe(true);
      expect(screen.fgAt(0, 0)).toBe("#0000ff");
    });

    test("reset clears previous styles", () => {
      const screen = new VirtualScreen(10, 5);
      // Set bold + color, then reset, then write
      const ansi = "\x1b[1;38;2;255;0;0mR\x1b[0mN";
      const data = new Uint8Array(Buffer.from(ansi));
      screen.apply(data);
      expect(screen.cellAt(0, 0)?.bold).toBe(true);
      expect(screen.fgAt(0, 0)).toBe("#ff0000");
      expect(screen.cellAt(0, 1)?.bold).toBe(false);
      expect(screen.fgAt(0, 1)).toBeUndefined();
    });
  });

  // ==========================================================================
  // Node removal cleanup
  // ==========================================================================

  describe("node removal", () => {
    test("removed nodes no longer in layout", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 5 }}>
          <box key="a" style={{ width: 10, height: 2 }}><text>A</text></box>
          <box key="b" style={{ width: 8, height: 2 }}><text>B</text></box>
        </box>,
        { cols: 20, rows: 5 },
      );
      const layouts1 = result.getAllLayouts();
      const count1 = layouts1.size;

      await result.rerender(
        <box style={{ flexDirection: "column", width: 20, height: 5 }}>
          <box key="a" style={{ width: 10, height: 2 }}><text>A</text></box>
        </box>,
      );
      const layouts2 = result.getAllLayouts();
      expect(layouts2.size).toBeLessThan(count1);
    });

    test("removed text no longer on screen", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}><text>Bye</text></box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("Bye");

      const screen2 = await result.rerender(
        <box style={{ width: 20, height: 3 }} />,
      );
      expect(screen2.containsText("Bye")).toBe(false);
    });
  });
});
