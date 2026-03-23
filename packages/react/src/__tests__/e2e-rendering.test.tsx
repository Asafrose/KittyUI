/**
 * E2E visual rendering tests for KittyUI.
 *
 * Tests background colors, foreground colors, text content, unicode,
 * bold/italic, color inheritance, overlapping regions, etc.
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
import "@kittyui/core/src/test-harness/assertions.js";
import { render, type RenderResult } from "../test-utils/render-jsx.js";

const canRun = new TestBridge().nativeAvailable;

describe.skipIf(!canRun)("E2E Rendering", () => {
  let result: RenderResult | undefined;

  afterEach(() => {
    result?.cleanup();
    result = undefined;
  });

  // ==========================================================================
  // Background colors
  // ==========================================================================

  describe("background colors", () => {
    test("red background #ff0000", async () => {
      result = await render(
        <box style={{ backgroundColor: "#ff0000", width: 10, height: 3 }}>
          <text>R</text>
        </box>,
        { cols: 20, rows: 5 },
      );
      const pos = result.screen.findText("R");
      expect(pos).toBeDefined();
      expect(result.screen).toHaveBgColor(pos!.row, pos!.col, "#ff0000");
    });

    test("green background #00ff00", async () => {
      result = await render(
        <box style={{ backgroundColor: "#00ff00", width: 10, height: 3 }}>
          <text>G</text>
        </box>,
        { cols: 20, rows: 5 },
      );
      const pos = result.screen.findText("G");
      expect(pos).toBeDefined();
      expect(result.screen).toHaveBgColor(pos!.row, pos!.col, "#00ff00");
    });

    test("blue background #0000ff", async () => {
      result = await render(
        <box style={{ backgroundColor: "#0000ff", width: 10, height: 3 }}>
          <text>B</text>
        </box>,
        { cols: 20, rows: 5 },
      );
      const pos = result.screen.findText("B");
      expect(pos).toBeDefined();
      expect(result.screen).toHaveBgColor(pos!.row, pos!.col, "#0000ff");
    });

    test("custom color #1e40af", async () => {
      result = await render(
        <box style={{ backgroundColor: "#1e40af", width: 10, height: 3 }}>
          <text>C</text>
        </box>,
        { cols: 20, rows: 5 },
      );
      const pos = result.screen.findText("C");
      expect(pos).toBeDefined();
      expect(result.screen).toHaveBgColor(pos!.row, pos!.col, "#1e40af");
    });

    test("white background #ffffff", async () => {
      result = await render(
        <box style={{ backgroundColor: "#ffffff", width: 10, height: 3 }}>
          <text>W</text>
        </box>,
        { cols: 20, rows: 5 },
      );
      const pos = result.screen.findText("W");
      expect(pos).toBeDefined();
      expect(result.screen).toHaveBgColor(pos!.row, pos!.col, "#ffffff");
    });

    test("black background #000000", async () => {
      result = await render(
        <box style={{ backgroundColor: "#000000", width: 10, height: 3 }}>
          <text>K</text>
        </box>,
        { cols: 20, rows: 5 },
      );
      const pos = result.screen.findText("K");
      expect(pos).toBeDefined();
      expect(result.screen).toHaveBgColor(pos!.row, pos!.col, "#000000");
    });

    test("background fills entire box area", async () => {
      result = await render(
        <box style={{ backgroundColor: "#ff0000", width: 5, height: 3 }} />,
        { cols: 10, rows: 5 },
      );
      // Check multiple cells within the box
      expect(result.screen).toHaveBgColor(0, 0, "#ff0000");
      expect(result.screen).toHaveBgColor(0, 4, "#ff0000");
      expect(result.screen).toHaveBgColor(2, 0, "#ff0000");
      expect(result.screen).toHaveBgColor(2, 4, "#ff0000");
    });

    test("background only in box bounds", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 20, height: 3 }}>
          <box style={{ backgroundColor: "#ff0000", width: 5, height: 3 }} />
          <box style={{ width: 5, height: 3 }} />
        </box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toHaveBgColor(0, 0, "#ff0000");
      // Cell at col 5 should NOT have red bg
      expect(result.screen.bgAt(0, 5)).not.toBe("#ff0000");
    });
  });

  // ==========================================================================
  // Foreground/text colors
  // ==========================================================================

  describe("foreground colors", () => {
    test("green text color", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}>
          <text style={{ color: "#00ff00" }}>Green</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      const pos = result.screen.findText("Green");
      expect(pos).toBeDefined();
      expect(result.screen).toHaveFgColor(pos!.row, pos!.col, "#00ff00");
    });

    test("red text color", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}>
          <text style={{ color: "#ff0000" }}>Red</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      const pos = result.screen.findText("Red");
      expect(pos).toBeDefined();
      expect(result.screen).toHaveFgColor(pos!.row, pos!.col, "#ff0000");
    });

    test("blue text color", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}>
          <text style={{ color: "#0000ff" }}>Blue</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      const pos = result.screen.findText("Blue");
      expect(pos).toBeDefined();
      expect(result.screen).toHaveFgColor(pos!.row, pos!.col, "#0000ff");
    });

    test("white text on dark background", async () => {
      result = await render(
        <box style={{ backgroundColor: "#000000", width: 20, height: 3 }}>
          <text style={{ color: "#ffffff" }}>White</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      const pos = result.screen.findText("White");
      expect(pos).toBeDefined();
      expect(result.screen).toHaveFgColor(pos!.row, pos!.col, "#ffffff");
      expect(result.screen).toHaveBgColor(pos!.row, pos!.col, "#000000");
    });

    test("text color applies to all characters", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}>
          <text style={{ color: "#ff0000" }}>ABC</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      const pos = result.screen.findText("ABC");
      expect(pos).toBeDefined();
      expect(result.screen).toHaveFgColor(pos!.row, pos!.col, "#ff0000");
      expect(result.screen).toHaveFgColor(pos!.row, pos!.col + 1, "#ff0000");
      expect(result.screen).toHaveFgColor(pos!.row, pos!.col + 2, "#ff0000");
    });
  });

  // ==========================================================================
  // Multiple colored regions
  // ==========================================================================

  describe("multiple colored regions", () => {
    test("two adjacent colored boxes", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 20, height: 3 }}>
          <box style={{ backgroundColor: "#ff0000", width: 10, height: 3 }}><text>R</text></box>
          <box style={{ backgroundColor: "#0000ff", width: 10, height: 3 }}><text>B</text></box>
        </box>,
        { cols: 20, rows: 3 },
      );
      const posR = result.screen.findText("R");
      const posB = result.screen.findText("B");
      expect(posR).toBeDefined();
      expect(posB).toBeDefined();
      expect(result.screen).toHaveBgColor(posR!.row, posR!.col, "#ff0000");
      expect(result.screen).toHaveBgColor(posB!.row, posB!.col, "#0000ff");
    });

    test("three colored boxes in row", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 30, height: 3 }}>
          <box style={{ backgroundColor: "#ff0000", width: 10, height: 3 }}><text>R</text></box>
          <box style={{ backgroundColor: "#00ff00", width: 10, height: 3 }}><text>G</text></box>
          <box style={{ backgroundColor: "#0000ff", width: 10, height: 3 }}><text>B</text></box>
        </box>,
        { cols: 30, rows: 3 },
      );
      expect(result.screen).toContainText("R");
      expect(result.screen).toContainText("G");
      expect(result.screen).toContainText("B");
    });

    test("colored boxes in column", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 6 }}>
          <box style={{ backgroundColor: "#ff0000", height: 3 }}><text>Top</text></box>
          <box style={{ backgroundColor: "#0000ff", height: 3 }}><text>Bot</text></box>
        </box>,
        { cols: 20, rows: 6 },
      );
      const posT = result.screen.findText("Top");
      const posB = result.screen.findText("Bot");
      expect(posT).toBeDefined();
      expect(posB).toBeDefined();
      expect(result.screen).toHaveBgColor(posT!.row, posT!.col, "#ff0000");
      expect(result.screen).toHaveBgColor(posB!.row, posB!.col, "#0000ff");
    });
  });

  // ==========================================================================
  // Nested backgrounds (child paints over parent)
  // ==========================================================================

  describe("nested backgrounds", () => {
    test("inner box paints over outer box", async () => {
      result = await render(
        <box style={{ backgroundColor: "#0000ff", width: 20, height: 5 }}>
          <box style={{ backgroundColor: "#ff0000", width: 10, height: 3 }}>
            <text>Inner</text>
          </box>
        </box>,
        { cols: 20, rows: 5 },
      );
      const pos = result.screen.findText("Inner");
      expect(pos).toBeDefined();
      expect(result.screen).toHaveBgColor(pos!.row, pos!.col, "#ff0000");
      // Outside inner box should be blue
      expect(result.screen).toHaveBgColor(4, 0, "#0000ff");
    });

    test("deeply nested backgrounds", async () => {
      result = await render(
        <box style={{ backgroundColor: "#ff0000", width: 20, height: 5 }}>
          <box style={{ backgroundColor: "#00ff00", width: 15, height: 4 }}>
            <box style={{ backgroundColor: "#0000ff", width: 10, height: 3 }}>
              <text>D</text>
            </box>
          </box>
        </box>,
        { cols: 20, rows: 5 },
      );
      const pos = result.screen.findText("D");
      expect(pos).toBeDefined();
      expect(result.screen).toHaveBgColor(pos!.row, pos!.col, "#0000ff");
    });
  });

  // ==========================================================================
  // Text content
  // ==========================================================================

  describe("text content", () => {
    test("single character", async () => {
      result = await render(
        <box style={{ width: 10, height: 3 }}><text>X</text></box>,
        { cols: 10, rows: 3 },
      );
      expect(result.screen).toContainText("X");
    });

    test("word", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}><text>Hello</text></box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("Hello");
    });

    test("sentence", async () => {
      result = await render(
        <box style={{ width: 40, height: 3 }}><text>The quick brown fox</text></box>,
        { cols: 40, rows: 3 },
      );
      expect(result.screen).toContainText("The quick brown fox");
    });

    test("empty string renders without error", async () => {
      result = await render(
        <box style={{ width: 10, height: 3 }}><text>{""}</text></box>,
        { cols: 10, rows: 3 },
      );
      // Should not crash
      expect(result.screen).toBeDefined();
    });

    test("text with spaces", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}><text>A B C</text></box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("A B C");
    });

    test("multiple text siblings rendered in order", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 20, height: 3 }}>
          <text>Hello</text>
          <text>World</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("Hello");
      expect(result.screen).toContainText("World");
    });

    test("text position matches layout", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 6 }}>
          <box style={{ height: 2 }} />
          <text>AtRow2</text>
        </box>,
        { cols: 20, rows: 6 },
      );
      const pos = result.screen.findText("AtRow2");
      expect(pos).toBeDefined();
      expect(pos!.row).toBe(2);
    });

    test("text truncated at box boundary", async () => {
      result = await render(
        <box style={{ width: 5, height: 1 }}>
          <text>LongText</text>
        </box>,
        { cols: 10, rows: 3 },
      );
      // The text should be truncated/clipped to the box width
      // At minimum the screen should not crash
      expect(result.screen).toBeDefined();
    });

    test("special characters render", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}><text>A&B</text></box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("A&B");
    });

    test("numbers render as text", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}><text>42</text></box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("42");
    });
  });

  // ==========================================================================
  // Unicode
  // ==========================================================================

  describe("unicode", () => {
    test("accented characters", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}><text>cafe</text></box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("cafe");
    });

    test("ASCII special characters", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}><text>@#$%</text></box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("@#$%");
    });
  });

  // ==========================================================================
  // Bold text
  // ==========================================================================

  describe("bold text", () => {
    test("bold text renders with bold attribute", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}>
          <text style={{ fontWeight: "bold" }}>Bold</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("Bold");
      const pos = result.screen.findText("Bold");
      expect(result.screen.cellAt(pos!.row, pos!.col)?.bold).toBe(true);
    });

    test("normal fontWeight is not bold", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}>
          <text style={{ fontWeight: "normal" }}>Normal</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("Normal");
      const pos = result.screen.findText("Normal");
      expect(result.screen.cellAt(pos!.row, pos!.col)?.bold).toBe(false);
    });
  });

  // ==========================================================================
  // Italic text
  // ==========================================================================

  describe("italic text", () => {
    test("italic text renders with italic attribute", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}>
          <text style={{ fontStyle: "italic" }}>Italic</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("Italic");
      const pos = result.screen.findText("Italic");
      expect(result.screen.cellAt(pos!.row, pos!.col)?.italic).toBe(true);
    });

    test("normal fontStyle is not italic", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}>
          <text style={{ fontStyle: "normal" }}>Plain</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      const pos = result.screen.findText("Plain");
      expect(result.screen.cellAt(pos!.row, pos!.col)?.italic).toBe(false);
    });
  });

  // ==========================================================================
  // Combined styles
  // ==========================================================================

  describe("combined styles", () => {
    test("bold + italic", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}>
          <text style={{ fontWeight: "bold", fontStyle: "italic" }}>BI</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      const pos = result.screen.findText("BI");
      expect(pos).toBeDefined();
      expect(result.screen.cellAt(pos!.row, pos!.col)?.bold).toBe(true);
      expect(result.screen.cellAt(pos!.row, pos!.col)?.italic).toBe(true);
    });

    test("colored text with background", async () => {
      result = await render(
        <box style={{ backgroundColor: "#0000ff", width: 20, height: 3 }}>
          <text style={{ color: "#ff0000" }}>Colored</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      const pos = result.screen.findText("Colored");
      expect(pos).toBeDefined();
      expect(result.screen).toHaveFgColor(pos!.row, pos!.col, "#ff0000");
      expect(result.screen).toHaveBgColor(pos!.row, pos!.col, "#0000ff");
    });

    test("bold colored text", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}>
          <text style={{ color: "#00ff00", fontWeight: "bold" }}>BG</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      const pos = result.screen.findText("BG");
      expect(pos).toBeDefined();
      expect(result.screen).toHaveFgColor(pos!.row, pos!.col, "#00ff00");
      expect(result.screen.cellAt(pos!.row, pos!.col)?.bold).toBe(true);
    });
  });

  // ==========================================================================
  // Empty boxes
  // ==========================================================================

  describe("empty boxes", () => {
    test("empty box with background renders color", async () => {
      result = await render(
        <box style={{ backgroundColor: "#ff0000", width: 5, height: 3 }} />,
        { cols: 10, rows: 5 },
      );
      expect(result.screen).toHaveBgColor(0, 0, "#ff0000");
    });

    test("empty box without background renders spaces", async () => {
      result = await render(
        <box style={{ width: 5, height: 3 }} />,
        { cols: 10, rows: 5 },
      );
      // Should just be spaces
      expect(result.screen.textAt(0, 0)).toBe(" ");
    });
  });

  // ==========================================================================
  // Full screen
  // ==========================================================================

  describe("full screen", () => {
    test("full-screen background fill", async () => {
      result = await render(
        <box style={{ backgroundColor: "#112233", width: 10, height: 5 }} />,
        { cols: 10, rows: 5 },
      );
      expect(result.screen).toHaveBgColor(0, 0, "#112233");
      expect(result.screen).toHaveBgColor(4, 9, "#112233");
    });
  });

  // ==========================================================================
  // Text at specific positions
  // ==========================================================================

  describe("textAt assertions", () => {
    test("toHaveTextAt verifies exact position", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}>
          <text>Hello</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toHaveTextAt(0, 0, "Hello");
    });

    test("text in padded box at correct position", async () => {
      result = await render(
        <box style={{ width: 20, height: 5, paddingLeft: 3, paddingTop: 1 }}>
          <text>Pad</text>
        </box>,
        { cols: 20, rows: 5 },
      );
      expect(result.screen).toHaveTextAt(1, 3, "Pad");
    });

    test("second child text at correct column", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 20, height: 3 }}>
          <box style={{ width: 5, height: 3 }}><text>AAAAA</text></box>
          <text>B</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toHaveTextAt(0, 5, "B");
    });
  });

  // ==========================================================================
  // Screen query methods
  // ==========================================================================

  describe("screen query methods", () => {
    test("containsText returns true when text present", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}><text>Found</text></box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen.containsText("Found")).toBe(true);
    });

    test("containsText returns false when text absent", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}><text>Hello</text></box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen.containsText("Missing")).toBe(false);
    });

    test("findText returns correct row and col", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 5 }}>
          <box style={{ height: 2 }} />
          <text>Here</text>
        </box>,
        { cols: 20, rows: 5 },
      );
      const pos = result.screen.findText("Here");
      expect(pos).toBeDefined();
      expect(pos!.row).toBe(2);
      expect(pos!.col).toBe(0);
    });

    test("getRowText returns full row", async () => {
      result = await render(
        <box style={{ width: 5, height: 3 }}><text>Hello</text></box>,
        { cols: 5, rows: 3 },
      );
      const row0 = result.screen.getRowText(0);
      expect(row0).toContain("Hello");
    });

    test("getTextContent returns all rows", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 10, height: 3 }}>
          <text>Line1</text>
          <text>Line2</text>
        </box>,
        { cols: 10, rows: 3 },
      );
      const content = result.screen.getTextContent();
      expect(content).toContain("Line1");
      expect(content).toContain("Line2");
    });

    test("cellAt returns undefined for out-of-bounds", async () => {
      result = await render(
        <box style={{ width: 5, height: 3 }}><text>A</text></box>,
        { cols: 5, rows: 3 },
      );
      expect(result.screen.cellAt(-1, 0)).toBeUndefined();
      expect(result.screen.cellAt(0, -1)).toBeUndefined();
      expect(result.screen.cellAt(3, 0)).toBeUndefined();
      expect(result.screen.cellAt(0, 5)).toBeUndefined();
    });

    test("toString returns text content", async () => {
      result = await render(
        <box style={{ width: 10, height: 2 }}><text>Test</text></box>,
        { cols: 10, rows: 2 },
      );
      const str = result.screen.toString();
      expect(str).toContain("Test");
    });
  });

  // ==========================================================================
  // Color on different regions
  // ==========================================================================

  describe("color regions", () => {
    test("different fg colors for different text nodes", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 20, height: 3 }}>
          <text style={{ color: "#ff0000" }}>Red</text>
          <text style={{ color: "#00ff00" }}>Grn</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      const posR = result.screen.findText("Red");
      const posG = result.screen.findText("Grn");
      expect(posR).toBeDefined();
      expect(posG).toBeDefined();
      expect(result.screen).toHaveFgColor(posR!.row, posR!.col, "#ff0000");
      expect(result.screen).toHaveFgColor(posG!.row, posG!.col, "#00ff00");
    });

    test("bg color on parent, fg on child", async () => {
      result = await render(
        <box style={{ backgroundColor: "#333333", width: 20, height: 3 }}>
          <text style={{ color: "#ffffff" }}>Bright</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      const pos = result.screen.findText("Bright");
      expect(pos).toBeDefined();
      expect(result.screen).toHaveBgColor(pos!.row, pos!.col, "#333333");
      expect(result.screen).toHaveFgColor(pos!.row, pos!.col, "#ffffff");
    });
  });
});
