/**
 * E2E style system tests for KittyUI.
 *
 * Tests CSS-like style properties: flexDirection, flexGrow,
 * backgroundColor, color, fontWeight/fontStyle, display,
 * style updates via rerender, and Color objects.
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

const findLayoutBySize = (
  layouts: Map<number, { x: number; y: number; width: number; height: number }>,
  w: number,
  h: number,
): { nodeId: number; x: number; y: number; width: number; height: number } | undefined => {
  for (const [nodeId, layout] of layouts) {
    if (Math.abs(layout.width - w) < 1 && Math.abs(layout.height - h) < 1) {
      return { nodeId, ...layout };
    }
  }
  return undefined;
};

describe.skipIf(!canRun)("E2E Styles", () => {
  let result: RenderResult | undefined;

  afterEach(() => {
    result?.cleanup();
    result = undefined;
  });

  // ==========================================================================
  // flexDirection values
  // ==========================================================================

  describe("flexDirection values", () => {
    test("row horizontal layout", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 20, height: 3 }}>
          <text>A</text><text>B</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      const a = result.screen.findText("A");
      const b = result.screen.findText("B");
      expect(a!.row).toBe(b!.row);
      expect(b!.col).toBeGreaterThan(a!.col);
    });

    test("column vertical layout", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 5 }}>
          <text>A</text><text>B</text>
        </box>,
        { cols: 20, rows: 5 },
      );
      const a = result.screen.findText("A");
      const b = result.screen.findText("B");
      expect(b!.row).toBeGreaterThan(a!.row);
    });

    test("row-reverse", async () => {
      result = await render(
        <box style={{ flexDirection: "row-reverse", width: 20, height: 3 }}>
          <text>A</text><text>B</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      const a = result.screen.findText("A");
      const b = result.screen.findText("B");
      expect(a!.col).toBeGreaterThan(b!.col);
    });

    test("column-reverse", async () => {
      result = await render(
        <box style={{ flexDirection: "column-reverse", width: 20, height: 5 }}>
          <text>A</text><text>B</text>
        </box>,
        { cols: 20, rows: 5 },
      );
      const a = result.screen.findText("A");
      const b = result.screen.findText("B");
      expect(a!.row).toBeGreaterThan(b!.row);
    });
  });

  // ==========================================================================
  // backgroundColor
  // ==========================================================================

  describe("backgroundColor", () => {
    test("string color #ff0000", async () => {
      result = await render(
        <box style={{ backgroundColor: "#ff0000", width: 5, height: 3 }}><text>S</text></box>,
        { cols: 10, rows: 5 },
      );
      const pos = result.screen.findText("S");
      expect(result.screen).toHaveBgColor(pos!.row, pos!.col, "#ff0000");
    });

    test("string color #00ff00", async () => {
      result = await render(
        <box style={{ backgroundColor: "#00ff00", width: 5, height: 3 }}><text>G</text></box>,
        { cols: 10, rows: 5 },
      );
      const pos = result.screen.findText("G");
      expect(result.screen).toHaveBgColor(pos!.row, pos!.col, "#00ff00");
    });

    test("string color #0000ff", async () => {
      result = await render(
        <box style={{ backgroundColor: "#0000ff", width: 5, height: 3 }}><text>B</text></box>,
        { cols: 10, rows: 5 },
      );
      const pos = result.screen.findText("B");
      expect(result.screen).toHaveBgColor(pos!.row, pos!.col, "#0000ff");
    });

    test("Color object", async () => {
      result = await render(
        <box style={{ backgroundColor: { type: "rgb", r: 0, g: 255, b: 0 }, width: 5, height: 3 }}>
          <text>O</text>
        </box>,
        { cols: 10, rows: 5 },
      );
      const pos = result.screen.findText("O");
      expect(result.screen).toHaveBgColor(pos!.row, pos!.col, "#00ff00");
    });

    test("custom hex #1e40af", async () => {
      result = await render(
        <box style={{ backgroundColor: "#1e40af", width: 5, height: 3 }}><text>C</text></box>,
        { cols: 10, rows: 5 },
      );
      const pos = result.screen.findText("C");
      expect(result.screen).toHaveBgColor(pos!.row, pos!.col, "#1e40af");
    });

    test("white #ffffff", async () => {
      result = await render(
        <box style={{ backgroundColor: "#ffffff", width: 5, height: 3 }}><text>W</text></box>,
        { cols: 10, rows: 5 },
      );
      const pos = result.screen.findText("W");
      expect(result.screen).toHaveBgColor(pos!.row, pos!.col, "#ffffff");
    });

    test("black #000000", async () => {
      result = await render(
        <box style={{ backgroundColor: "#000000", width: 5, height: 3 }}><text>K</text></box>,
        { cols: 10, rows: 5 },
      );
      const pos = result.screen.findText("K");
      expect(result.screen).toHaveBgColor(pos!.row, pos!.col, "#000000");
    });

    test("fills entire box", async () => {
      result = await render(
        <box style={{ backgroundColor: "#ff0000", width: 5, height: 3 }} />,
        { cols: 10, rows: 5 },
      );
      expect(result.screen).toHaveBgColor(0, 0, "#ff0000");
      expect(result.screen).toHaveBgColor(0, 4, "#ff0000");
      expect(result.screen).toHaveBgColor(2, 0, "#ff0000");
    });
  });

  // ==========================================================================
  // color (foreground)
  // ==========================================================================

  describe("color (foreground)", () => {
    test("string color", async () => {
      result = await render(
        <box style={{ width: 10, height: 3 }}>
          <text style={{ color: "#ff0000" }}>T</text>
        </box>,
        { cols: 10, rows: 3 },
      );
      const pos = result.screen.findText("T");
      expect(result.screen).toHaveFgColor(pos!.row, pos!.col, "#ff0000");
    });

    test("Color object", async () => {
      result = await render(
        <box style={{ width: 10, height: 3 }}>
          <text style={{ color: { type: "rgb", r: 0, g: 0, b: 255 } }}>T</text>
        </box>,
        { cols: 10, rows: 3 },
      );
      const pos = result.screen.findText("T");
      expect(result.screen).toHaveFgColor(pos!.row, pos!.col, "#0000ff");
    });

    test("green text", async () => {
      result = await render(
        <box style={{ width: 10, height: 3 }}>
          <text style={{ color: "#00ff00" }}>G</text>
        </box>,
        { cols: 10, rows: 3 },
      );
      const pos = result.screen.findText("G");
      expect(result.screen).toHaveFgColor(pos!.row, pos!.col, "#00ff00");
    });

    test("applies to all chars", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}>
          <text style={{ color: "#ff0000" }}>ABC</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      const pos = result.screen.findText("ABC");
      expect(result.screen).toHaveFgColor(pos!.row, pos!.col, "#ff0000");
      expect(result.screen).toHaveFgColor(pos!.row, pos!.col + 1, "#ff0000");
      expect(result.screen).toHaveFgColor(pos!.row, pos!.col + 2, "#ff0000");
    });

    test("fg on bg", async () => {
      result = await render(
        <box style={{ backgroundColor: "#000000", width: 20, height: 3 }}>
          <text style={{ color: "#ffffff" }}>W</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      const pos = result.screen.findText("W");
      expect(result.screen).toHaveFgColor(pos!.row, pos!.col, "#ffffff");
      expect(result.screen).toHaveBgColor(pos!.row, pos!.col, "#000000");
    });
  });

  // ==========================================================================
  // fontWeight / fontStyle (render content, no bold/italic cell assertion)
  // ==========================================================================

  describe("fontWeight", () => {
    test("bold renders content", async () => {
      result = await render(
        <box style={{ width: 10, height: 3 }}>
          <text style={{ fontWeight: "bold" }}>B</text>
        </box>,
        { cols: 10, rows: 3 },
      );
      expect(result.screen).toContainText("B");
    });

    test("normal renders content", async () => {
      result = await render(
        <box style={{ width: 10, height: 3 }}>
          <text style={{ fontWeight: "normal" }}>N</text>
        </box>,
        { cols: 10, rows: 3 },
      );
      expect(result.screen).toContainText("N");
    });
  });

  describe("fontStyle", () => {
    test("italic renders content", async () => {
      result = await render(
        <box style={{ width: 10, height: 3 }}>
          <text style={{ fontStyle: "italic" }}>I</text>
        </box>,
        { cols: 10, rows: 3 },
      );
      expect(result.screen).toContainText("I");
    });

    test("normal renders content", async () => {
      result = await render(
        <box style={{ width: 10, height: 3 }}>
          <text style={{ fontStyle: "normal" }}>N</text>
        </box>,
        { cols: 10, rows: 3 },
      );
      expect(result.screen).toContainText("N");
    });
  });

  // ==========================================================================
  // Style updates via rerender
  // ==========================================================================

  describe("style updates", () => {
    test("change backgroundColor on rerender", async () => {
      result = await render(
        <box style={{ backgroundColor: "#ff0000", width: 10, height: 3 }}><text>X</text></box>,
        { cols: 20, rows: 5 },
      );
      const pos1 = result.screen.findText("X");
      expect(result.screen).toHaveBgColor(pos1!.row, pos1!.col, "#ff0000");

      const screen2 = await result.rerender(
        <box style={{ backgroundColor: "#00ff00", width: 10, height: 3 }}><text>X</text></box>,
      );
      const pos2 = screen2.findText("X");
      expect(screen2).toHaveBgColor(pos2!.row, pos2!.col, "#00ff00");
    });

    test("change dimensions on rerender", async () => {
      result = await render(
        <box style={{ width: 10, height: 3 }}><text>A</text></box>,
        { cols: 40, rows: 10 },
      );
      expect(findLayoutBySize(result.getAllLayouts(), 10, 3)).toBeDefined();

      await result.rerender(
        <box style={{ width: 20, height: 5 }}><text>A</text></box>,
      );
      expect(findLayoutBySize(result.getAllLayouts(), 20, 5)).toBeDefined();
    });

    test("change flexDirection on rerender updates layout", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 20, height: 5 }}>
          <box style={{ width: 5, height: 2 }}><text>A</text></box>
          <box style={{ width: 5, height: 2 }}><text>B</text></box>
        </box>,
        { cols: 20, rows: 5 },
      );
      const a1 = result.screen.findText("A");
      const b1 = result.screen.findText("B");
      expect(a1!.row).toBe(b1!.row);

      await result.rerender(
        <box style={{ flexDirection: "column", width: 20, height: 5 }}>
          <box style={{ width: 5, height: 2 }}><text>A</text></box>
          <box style={{ width: 5, height: 2 }}><text>B</text></box>
        </box>,
      );
      const layouts = result.getAllLayouts();
      expect(layouts.size).toBeGreaterThanOrEqual(4);
    });

    test("change text color on rerender", async () => {
      result = await render(
        <box style={{ width: 10, height: 3 }}>
          <text style={{ color: "#ff0000" }}>T</text>
        </box>,
        { cols: 10, rows: 3 },
      );
      const pos1 = result.screen.findText("T");
      expect(result.screen).toHaveFgColor(pos1!.row, pos1!.col, "#ff0000");

      const screen2 = await result.rerender(
        <box style={{ width: 10, height: 3 }}>
          <text style={{ color: "#0000ff" }}>T</text>
        </box>,
      );
      const pos2 = screen2.findText("T");
      expect(screen2).toHaveFgColor(pos2!.row, pos2!.col, "#0000ff");
    });

    test("add color on rerender", async () => {
      result = await render(
        <box style={{ width: 10, height: 3 }}><text>T</text></box>,
        { cols: 10, rows: 3 },
      );
      expect(result.screen.fgAt(0, 0)).toBeUndefined();

      const screen2 = await result.rerender(
        <box style={{ width: 10, height: 3 }}>
          <text style={{ color: "#ff0000" }}>T</text>
        </box>,
      );
      const pos = screen2.findText("T");
      expect(screen2).toHaveFgColor(pos!.row, pos!.col, "#ff0000");
    });

    test("remove backgroundColor on rerender", async () => {
      result = await render(
        <box style={{ backgroundColor: "#ff0000", width: 10, height: 3 }}><text>X</text></box>,
        { cols: 20, rows: 5 },
      );
      const pos1 = result.screen.findText("X");
      expect(result.screen).toHaveBgColor(pos1!.row, pos1!.col, "#ff0000");

      const screen2 = await result.rerender(
        <box style={{ width: 10, height: 3 }}><text>X</text></box>,
      );
      const pos2 = screen2.findText("X");
      expect(pos2).toBeDefined();
      // bg should no longer be red
      expect(screen2.bgAt(pos2!.row, pos2!.col)).not.toBe("#ff0000");
    });
  });

  // ==========================================================================
  // Display
  // ==========================================================================

  describe("display", () => {
    test("flex is default display", async () => {
      result = await render(
        <box style={{ width: 10, height: 3 }}><text>D</text></box>,
        { cols: 10, rows: 3 },
      );
      expect(result.screen).toContainText("D");
    });

    test("explicit display: flex", async () => {
      result = await render(
        <box style={{ display: "flex", width: 10, height: 3 }}><text>F</text></box>,
        { cols: 10, rows: 3 },
      );
      expect(result.screen).toContainText("F");
    });
  });

  // ==========================================================================
  // Multiple colored regions
  // ==========================================================================

  describe("multiple colored regions", () => {
    test("two adjacent colored boxes in row", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 20, height: 3 }}>
          <box style={{ backgroundColor: "#ff0000", width: 10, height: 3 }}><text>R</text></box>
          <box style={{ backgroundColor: "#0000ff", width: 10, height: 3 }}><text>B</text></box>
        </box>,
        { cols: 20, rows: 3 },
      );
      const posR = result.screen.findText("R");
      const posB = result.screen.findText("B");
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
      expect(result.screen).toHaveBgColor(posT!.row, posT!.col, "#ff0000");
      expect(result.screen).toHaveBgColor(posB!.row, posB!.col, "#0000ff");
    });

    test("nested backgrounds (inner paints over outer)", async () => {
      result = await render(
        <box style={{ backgroundColor: "#0000ff", width: 20, height: 5 }}>
          <box style={{ backgroundColor: "#ff0000", width: 10, height: 3 }}>
            <text>I</text>
          </box>
        </box>,
        { cols: 20, rows: 5 },
      );
      const pos = result.screen.findText("I");
      expect(result.screen).toHaveBgColor(pos!.row, pos!.col, "#ff0000");
      expect(result.screen).toHaveBgColor(4, 0, "#0000ff");
    });

    test("different fg colors on siblings", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 20, height: 3 }}>
          <text style={{ color: "#ff0000" }}>Red</text>
          <text style={{ color: "#00ff00" }}>Grn</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      const posR = result.screen.findText("Red");
      const posG = result.screen.findText("Grn");
      expect(result.screen).toHaveFgColor(posR!.row, posR!.col, "#ff0000");
      expect(result.screen).toHaveFgColor(posG!.row, posG!.col, "#00ff00");
    });
  });

  // ==========================================================================
  // Stretch (default cross-axis)
  // ==========================================================================

  describe("stretch", () => {
    test("child stretches to fill cross-axis in row", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 20, height: 6 }}>
          <box style={{ width: 10 }}><text>S</text></box>
        </box>,
        { cols: 20, rows: 6 },
      );
      const layouts = result.getAllLayouts();
      expect(findLayoutBySize(layouts, 10, 6)).toBeDefined();
    });

    test("child stretches to fill cross-axis in column", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 6 }}>
          <box style={{ height: 3 }}><text>S</text></box>
        </box>,
        { cols: 20, rows: 6 },
      );
      const layouts = result.getAllLayouts();
      expect(findLayoutBySize(layouts, 20, 3)).toBeDefined();
    });
  });
});
