/**
 * E2E VirtualScreen and ANSI parser tests for KittyUI.
 *
 * Tests screen construction, cell queries, text search,
 * ANSI parsing (CUP, SGR), style state management,
 * and integration with the render pipeline.
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
import "@kittyui/core/src/test-harness/assertions.js";
import { render, type RenderResult } from "../test-utils/render-jsx.js";

const canRun = new TestBridge().nativeAvailable;

describe("VirtualScreen unit tests", () => {
  // ==========================================================================
  // Construction
  // ==========================================================================

  describe("construction", () => {
    test("creates screen with correct dimensions", () => {
      const screen = new VirtualScreen(10, 5);
      expect(screen.cols).toBe(10);
      expect(screen.rows).toBe(5);
    });

    test("1x1 screen", () => {
      const screen = new VirtualScreen(1, 1);
      expect(screen.cols).toBe(1);
      expect(screen.rows).toBe(1);
      expect(screen.textAt(0, 0)).toBe(" ");
    });

    test("large screen 80x24", () => {
      const screen = new VirtualScreen(80, 24);
      expect(screen.cols).toBe(80);
      expect(screen.rows).toBe(24);
    });

    test("initial cells are spaces", () => {
      const screen = new VirtualScreen(5, 3);
      for (let r = 0; r < 3; r++) {
        for (let c = 0; c < 5; c++) {
          expect(screen.textAt(r, c)).toBe(" ");
        }
      }
    });

    test("initial cells have no fg color", () => {
      const screen = new VirtualScreen(5, 3);
      expect(screen.fgAt(0, 0)).toBeUndefined();
    });

    test("initial cells have no bg color", () => {
      const screen = new VirtualScreen(5, 3);
      expect(screen.bgAt(0, 0)).toBeUndefined();
    });

    test("initial cells are not bold", () => {
      const screen = new VirtualScreen(5, 3);
      expect(screen.cellAt(0, 0)?.bold).toBe(false);
    });

    test("initial cells are not italic", () => {
      const screen = new VirtualScreen(5, 3);
      expect(screen.cellAt(0, 0)?.italic).toBe(false);
    });
  });

  // ==========================================================================
  // cellAt
  // ==========================================================================

  describe("cellAt", () => {
    test("returns cell for valid position", () => {
      const screen = new VirtualScreen(5, 3);
      const cell = screen.cellAt(0, 0);
      expect(cell).toBeDefined();
      expect(cell!.ch).toBe(" ");
    });

    test("returns undefined for negative row", () => {
      const screen = new VirtualScreen(5, 3);
      expect(screen.cellAt(-1, 0)).toBeUndefined();
    });

    test("returns undefined for negative col", () => {
      const screen = new VirtualScreen(5, 3);
      expect(screen.cellAt(0, -1)).toBeUndefined();
    });

    test("returns undefined for row >= rows", () => {
      const screen = new VirtualScreen(5, 3);
      expect(screen.cellAt(3, 0)).toBeUndefined();
    });

    test("returns undefined for col >= cols", () => {
      const screen = new VirtualScreen(5, 3);
      expect(screen.cellAt(0, 5)).toBeUndefined();
    });

    test("last valid cell", () => {
      const screen = new VirtualScreen(5, 3);
      expect(screen.cellAt(2, 4)).toBeDefined();
    });
  });

  // ==========================================================================
  // textAt
  // ==========================================================================

  describe("textAt", () => {
    test("returns space for empty cell", () => {
      const screen = new VirtualScreen(5, 3);
      expect(screen.textAt(0, 0)).toBe(" ");
    });

    test("returns undefined for out-of-bounds", () => {
      const screen = new VirtualScreen(5, 3);
      expect(screen.textAt(5, 0)).toBeUndefined();
    });

    test("returns written character", () => {
      const screen = new VirtualScreen(5, 3);
      screen.apply(new Uint8Array([0x41])); // 'A'
      expect(screen.textAt(0, 0)).toBe("A");
    });
  });

  // ==========================================================================
  // bgAt / fgAt
  // ==========================================================================

  describe("bgAt / fgAt", () => {
    test("bgAt returns undefined for no bg", () => {
      const screen = new VirtualScreen(5, 3);
      expect(screen.bgAt(0, 0)).toBeUndefined();
    });

    test("fgAt returns undefined for no fg", () => {
      const screen = new VirtualScreen(5, 3);
      expect(screen.fgAt(0, 0)).toBeUndefined();
    });

    test("bgAt returns color after SGR bg", () => {
      const screen = new VirtualScreen(5, 3);
      const ansi = "\x1b[48;2;255;0;0mX";
      screen.apply(new Uint8Array(Buffer.from(ansi)));
      expect(screen.bgAt(0, 0)).toBe("#ff0000");
    });

    test("fgAt returns color after SGR fg", () => {
      const screen = new VirtualScreen(5, 3);
      const ansi = "\x1b[38;2;0;255;0mX";
      screen.apply(new Uint8Array(Buffer.from(ansi)));
      expect(screen.fgAt(0, 0)).toBe("#00ff00");
    });
  });

  // ==========================================================================
  // findText
  // ==========================================================================

  describe("findText", () => {
    test("returns undefined when not found", () => {
      const screen = new VirtualScreen(10, 5);
      expect(screen.findText("hello")).toBeUndefined();
    });

    test("finds text at start of first row", () => {
      const screen = new VirtualScreen(10, 5);
      screen.apply(new Uint8Array(Buffer.from("Hello")));
      expect(screen.findText("Hello")).toEqual({ row: 0, col: 0 });
    });

    test("finds text at specific position", () => {
      const screen = new VirtualScreen(10, 5);
      const ansi = "\x1b[3;4HTest";
      screen.apply(new Uint8Array(Buffer.from(ansi)));
      const pos = screen.findText("Test");
      expect(pos).toEqual({ row: 2, col: 3 });
    });

    test("finds single character", () => {
      const screen = new VirtualScreen(10, 5);
      screen.apply(new Uint8Array([0x58])); // 'X'
      expect(screen.findText("X")).toEqual({ row: 0, col: 0 });
    });
  });

  // ==========================================================================
  // containsText
  // ==========================================================================

  describe("containsText", () => {
    test("returns false for empty screen", () => {
      const screen = new VirtualScreen(10, 5);
      expect(screen.containsText("hello")).toBe(false);
    });

    test("returns true when text exists", () => {
      const screen = new VirtualScreen(10, 5);
      screen.apply(new Uint8Array(Buffer.from("hello")));
      expect(screen.containsText("hello")).toBe(true);
    });

    test("returns false for partial non-match", () => {
      const screen = new VirtualScreen(10, 5);
      screen.apply(new Uint8Array(Buffer.from("hello")));
      expect(screen.containsText("helloX")).toBe(false);
    });

    test("spaces count as text", () => {
      const screen = new VirtualScreen(10, 5);
      // Default is spaces
      expect(screen.containsText("  ")).toBe(true);
    });
  });

  // ==========================================================================
  // getRowText
  // ==========================================================================

  describe("getRowText", () => {
    test("returns full row of spaces for empty screen", () => {
      const screen = new VirtualScreen(5, 3);
      expect(screen.getRowText(0)).toBe("     ");
    });

    test("returns empty string for out-of-bounds row", () => {
      const screen = new VirtualScreen(5, 3);
      expect(screen.getRowText(3)).toBe("");
      expect(screen.getRowText(-1)).toBe("");
    });

    test("returns row with content", () => {
      const screen = new VirtualScreen(10, 5);
      screen.apply(new Uint8Array(Buffer.from("Hello")));
      const row = screen.getRowText(0);
      expect(row.startsWith("Hello")).toBe(true);
    });
  });

  // ==========================================================================
  // getTextContent
  // ==========================================================================

  describe("getTextContent", () => {
    test("returns all rows joined by newlines", () => {
      const screen = new VirtualScreen(3, 2);
      const content = screen.getTextContent();
      expect(content).toBe("   \n   ");
    });

    test("includes content from apply", () => {
      const screen = new VirtualScreen(10, 2);
      screen.apply(new Uint8Array(Buffer.from("AB")));
      const content = screen.getTextContent();
      expect(content).toContain("AB");
    });
  });

  // ==========================================================================
  // toString
  // ==========================================================================

  describe("toString", () => {
    test("matches getTextContent", () => {
      const screen = new VirtualScreen(5, 2);
      expect(screen.toString()).toBe(screen.getTextContent());
    });
  });

  // ==========================================================================
  // ANSI CUP (cursor position)
  // ==========================================================================

  describe("CUP parsing", () => {
    test("ESC[H moves to 0,0", () => {
      const screen = new VirtualScreen(10, 5);
      // Write 'X' at current pos (0,0 by default)
      screen.apply(new Uint8Array(Buffer.from("\x1b[HX")));
      expect(screen.textAt(0, 0)).toBe("X");
    });

    test("ESC[1;1H moves to row 0, col 0", () => {
      const screen = new VirtualScreen(10, 5);
      screen.apply(new Uint8Array(Buffer.from("\x1b[1;1HA")));
      expect(screen.textAt(0, 0)).toBe("A");
    });

    test("ESC[3;5H moves to row 2, col 4", () => {
      const screen = new VirtualScreen(10, 5);
      screen.apply(new Uint8Array(Buffer.from("\x1b[3;5HB")));
      expect(screen.textAt(2, 4)).toBe("B");
    });

    test("sequential CUP moves", () => {
      const screen = new VirtualScreen(10, 5);
      screen.apply(new Uint8Array(Buffer.from("\x1b[1;1HA\x1b[5;10HZ")));
      expect(screen.textAt(0, 0)).toBe("A");
      expect(screen.textAt(4, 9)).toBe("Z");
    });
  });

  // ==========================================================================
  // ANSI SGR (set graphic rendition)
  // ==========================================================================

  describe("SGR parsing", () => {
    test("SGR 0 resets all attributes", () => {
      const screen = new VirtualScreen(10, 5);
      screen.apply(new Uint8Array(Buffer.from("\x1b[1m\x1b[0mN")));
      expect(screen.cellAt(0, 0)?.bold).toBe(false);
    });

    test("SGR 1 sets bold", () => {
      const screen = new VirtualScreen(10, 5);
      screen.apply(new Uint8Array(Buffer.from("\x1b[1mB")));
      expect(screen.cellAt(0, 0)?.bold).toBe(true);
    });

    test("SGR 3 sets italic", () => {
      const screen = new VirtualScreen(10, 5);
      screen.apply(new Uint8Array(Buffer.from("\x1b[3mI")));
      expect(screen.cellAt(0, 0)?.italic).toBe(true);
    });

    test("SGR 38;2;R;G;B sets fg RGB", () => {
      const screen = new VirtualScreen(10, 5);
      screen.apply(new Uint8Array(Buffer.from("\x1b[38;2;128;64;32mC")));
      expect(screen.fgAt(0, 0)).toBe("#804020");
    });

    test("SGR 48;2;R;G;B sets bg RGB", () => {
      const screen = new VirtualScreen(10, 5);
      screen.apply(new Uint8Array(Buffer.from("\x1b[48;2;10;20;30mC")));
      expect(screen.bgAt(0, 0)).toBe("#0a141e");
    });

    test("combined SGR in single sequence", () => {
      const screen = new VirtualScreen(10, 5);
      // bold + fg red
      screen.apply(new Uint8Array(Buffer.from("\x1b[1;38;2;255;0;0mX")));
      expect(screen.cellAt(0, 0)?.bold).toBe(true);
      expect(screen.fgAt(0, 0)).toBe("#ff0000");
    });

    test("SGR reset clears fg", () => {
      const screen = new VirtualScreen(10, 5);
      screen.apply(new Uint8Array(Buffer.from("\x1b[38;2;255;0;0mR\x1b[0mN")));
      expect(screen.fgAt(0, 0)).toBe("#ff0000");
      expect(screen.fgAt(0, 1)).toBeUndefined();
    });

    test("SGR reset clears bg", () => {
      const screen = new VirtualScreen(10, 5);
      screen.apply(new Uint8Array(Buffer.from("\x1b[48;2;0;255;0mG\x1b[0mN")));
      expect(screen.bgAt(0, 0)).toBe("#00ff00");
      expect(screen.bgAt(0, 1)).toBeUndefined();
    });

    test("SGR reset clears bold", () => {
      const screen = new VirtualScreen(10, 5);
      screen.apply(new Uint8Array(Buffer.from("\x1b[1mB\x1b[0mN")));
      expect(screen.cellAt(0, 0)?.bold).toBe(true);
      expect(screen.cellAt(0, 1)?.bold).toBe(false);
    });

    test("SGR reset clears italic", () => {
      const screen = new VirtualScreen(10, 5);
      screen.apply(new Uint8Array(Buffer.from("\x1b[3mI\x1b[0mN")));
      expect(screen.cellAt(0, 0)?.italic).toBe(true);
      expect(screen.cellAt(0, 1)?.italic).toBe(false);
    });
  });

  // ==========================================================================
  // Character overwrite
  // ==========================================================================

  describe("overwrite", () => {
    test("writing at same position overwrites", () => {
      const screen = new VirtualScreen(10, 5);
      screen.apply(new Uint8Array(Buffer.from("\x1b[1;1HA\x1b[1;1HB")));
      expect(screen.textAt(0, 0)).toBe("B");
    });

    test("overwrite with different style", () => {
      const screen = new VirtualScreen(10, 5);
      screen.apply(new Uint8Array(Buffer.from(
        "\x1b[38;2;255;0;0m\x1b[1;1HA" +
        "\x1b[38;2;0;255;0m\x1b[1;1HB"
      )));
      expect(screen.textAt(0, 0)).toBe("B");
      expect(screen.fgAt(0, 0)).toBe("#00ff00");
    });
  });
});

// ==========================================================================
// Integration with render pipeline
// ==========================================================================

describe.skipIf(!canRun)("VirtualScreen integration", () => {
  let result: RenderResult | undefined;

  afterEach(() => {
    result?.cleanup();
    result = undefined;
  });

  test("rendered screen has correct dimensions", async () => {
    result = await render(
      <box style={{ width: 20, height: 5 }}><text>X</text></box>,
      { cols: 20, rows: 5 },
    );
    expect(result.screen.cols).toBe(20);
    expect(result.screen.rows).toBe(5);
  });

  test("rendered text found by findText", async () => {
    result = await render(
      <box style={{ width: 20, height: 3 }}><text>FindMe</text></box>,
      { cols: 20, rows: 3 },
    );
    const pos = result.screen.findText("FindMe");
    expect(pos).toBeDefined();
    expect(pos!.row).toBe(0);
    expect(pos!.col).toBe(0);
  });

  test("background color accessible via bgAt", async () => {
    result = await render(
      <box style={{ backgroundColor: "#ff0000", width: 5, height: 3 }} />,
      { cols: 10, rows: 5 },
    );
    expect(result.screen.bgAt(0, 0)).toBe("#ff0000");
  });

  test("fg color accessible via fgAt", async () => {
    result = await render(
      <box style={{ width: 20, height: 3 }}>
        <text style={{ color: "#00ff00" }}>G</text>
      </box>,
      { cols: 20, rows: 3 },
    );
    const pos = result.screen.findText("G");
    expect(result.screen.fgAt(pos!.row, pos!.col)).toBe("#00ff00");
  });

  test("bold accessible via cellAt", async () => {
    result = await render(
      <box style={{ width: 20, height: 3 }}>
        <text style={{ fontWeight: "bold" }}>B</text>
      </box>,
      { cols: 20, rows: 3 },
    );
    const pos = result.screen.findText("B");
    expect(result.screen.cellAt(pos!.row, pos!.col)?.bold).toBe(true);
  });

  test("italic accessible via cellAt", async () => {
    result = await render(
      <box style={{ width: 20, height: 3 }}>
        <text style={{ fontStyle: "italic" }}>I</text>
      </box>,
      { cols: 20, rows: 3 },
    );
    const pos = result.screen.findText("I");
    expect(result.screen.cellAt(pos!.row, pos!.col)?.italic).toBe(true);
  });

  test("getTextContent after render", async () => {
    result = await render(
      <box style={{ width: 10, height: 2 }}><text>Hello</text></box>,
      { cols: 10, rows: 2 },
    );
    const content = result.screen.getTextContent();
    expect(content).toContain("Hello");
  });

  test("screen toString after render", async () => {
    result = await render(
      <box style={{ width: 10, height: 2 }}><text>Str</text></box>,
      { cols: 10, rows: 2 },
    );
    const str = result.screen.toString();
    expect(str).toContain("Str");
  });

  test("containsText on rendered screen", async () => {
    result = await render(
      <box style={{ width: 20, height: 3 }}><text>Present</text></box>,
      { cols: 20, rows: 3 },
    );
    expect(result.screen.containsText("Present")).toBe(true);
    expect(result.screen.containsText("Absent")).toBe(false);
  });
});
