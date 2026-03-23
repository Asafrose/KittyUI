/**
 * E2E layout engine tests for KittyUI.
 *
 * Tests flex layout: fixed dimensions, percentage dimensions, flexDirection,
 * flexGrow, flexBasis, padding, margin, gap, justifyContent, alignItems,
 * nested layouts, many children, min/max constraints, and edge cases.
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

const findAllLayoutsBySize = (
  layouts: Map<number, { x: number; y: number; width: number; height: number }>,
  w: number,
  h: number,
): { nodeId: number; x: number; y: number; width: number; height: number }[] => {
  const results: { nodeId: number; x: number; y: number; width: number; height: number }[] = [];
  for (const [nodeId, layout] of layouts) {
    if (Math.abs(layout.width - w) < 1 && Math.abs(layout.height - h) < 1) {
      results.push({ nodeId, ...layout });
    }
  }
  return results;
};

describe.skipIf(!canRun)("E2E Layout", () => {
  let result: RenderResult | undefined;

  afterEach(() => {
    result?.cleanup();
    result = undefined;
  });

  // ==========================================================================
  // Fixed dimensions
  // ==========================================================================

  describe("fixed dimensions", () => {
    test("width 5, height 3", async () => {
      result = await render(<box style={{ width: 5, height: 3 }}><text>A</text></box>, { cols: 40, rows: 10 });
      expect(findLayoutBySize(result.getAllLayouts(), 5, 3)).toBeDefined();
    });

    test("width 10, height 5", async () => {
      result = await render(<box style={{ width: 10, height: 5 }}><text>B</text></box>, { cols: 40, rows: 10 });
      expect(findLayoutBySize(result.getAllLayouts(), 10, 5)).toBeDefined();
    });

    test("width 20, height 8", async () => {
      result = await render(<box style={{ width: 20, height: 8 }}><text>C</text></box>, { cols: 40, rows: 10 });
      expect(findLayoutBySize(result.getAllLayouts(), 20, 8)).toBeDefined();
    });

    test("width 40, height 10 fills viewport", async () => {
      result = await render(<box style={{ width: 40, height: 10 }}><text>D</text></box>, { cols: 40, rows: 10 });
      expect(findLayoutBySize(result.getAllLayouts(), 40, 10)).toBeDefined();
    });

    test("width 1, height 1 minimal box", async () => {
      result = await render(<box style={{ width: 1, height: 1 }}><text>X</text></box>, { cols: 40, rows: 10 });
      expect(findLayoutBySize(result.getAllLayouts(), 1, 1)).toBeDefined();
    });

    test("width 80, height 24 large viewport", async () => {
      result = await render(<box style={{ width: 80, height: 24 }}><text>L</text></box>, { cols: 80, rows: 24 });
      expect(findLayoutBySize(result.getAllLayouts(), 80, 24)).toBeDefined();
    });
  });

  describe("percentage dimensions", () => {
    test("50% width in 40-col viewport resolves to 20", async () => {
      result = await render(<box style={{ width: "50%", height: 5 }}><text>half</text></box>, { cols: 40, rows: 10 });
      expect(findLayoutBySize(result.getAllLayouts(), 20, 5)).toBeDefined();
    });

    test("100% width fills parent", async () => {
      result = await render(<box style={{ width: "100%", height: 5 }}><text>full</text></box>, { cols: 40, rows: 10 });
      expect(findLayoutBySize(result.getAllLayouts(), 40, 5)).toBeDefined();
    });

    test("25% width in 40-col viewport resolves to 10", async () => {
      result = await render(<box style={{ width: "25%", height: 4 }}><text>q</text></box>, { cols: 40, rows: 10 });
      expect(findLayoutBySize(result.getAllLayouts(), 10, 4)).toBeDefined();
    });

    test("50% height in 10-row viewport resolves to 5", async () => {
      result = await render(<box style={{ width: 20, height: "50%" }}><text>h</text></box>, { cols: 40, rows: 10 });
      expect(findLayoutBySize(result.getAllLayouts(), 20, 5)).toBeDefined();
    });

    test("100% width and 100% height", async () => {
      result = await render(<box style={{ width: "100%", height: "100%" }}><text>all</text></box>, { cols: 30, rows: 8 });
      expect(findLayoutBySize(result.getAllLayouts(), 30, 8)).toBeDefined();
    });
  });

  describe("flexDirection", () => {
    test("row places children side by side", async () => {
      result = await render(<box style={{ flexDirection: "row", width: 20, height: 3 }}><text>AA</text><text>BB</text></box>, { cols: 20, rows: 3 });
      expect(result.screen.findText("BB")!.col).toBeGreaterThan(result.screen.findText("AA")!.col);
    });

    test("column stacks children vertically", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 6 }}>
          <box style={{ height: 2 }}><text>Top</text></box>
          <box style={{ height: 2 }}><text>Bot</text></box>
        </box>, { cols: 20, rows: 6 });
      expect(result.screen.findText("Bot")!.row).toBeGreaterThan(result.screen.findText("Top")!.row);
    });

    test("row-reverse reverses order", async () => {
      result = await render(<box style={{ flexDirection: "row-reverse", width: 20, height: 3 }}><text>AA</text><text>BB</text></box>, { cols: 20, rows: 3 });
      expect(result.screen.findText("AA")!.col).toBeGreaterThan(result.screen.findText("BB")!.col);
    });

    test("column-reverse reverses vertical order", async () => {
      result = await render(
        <box style={{ flexDirection: "column-reverse", width: 20, height: 6 }}>
          <box style={{ height: 2 }}><text>Top</text></box>
          <box style={{ height: 2 }}><text>Bot</text></box>
        </box>, { cols: 20, rows: 6 });
      expect(result.screen.findText("Top")!.row).toBeGreaterThan(result.screen.findText("Bot")!.row);
    });
  });

  describe("flexGrow", () => {
    test("single child with flexGrow fills parent", async () => {
      result = await render(<box style={{ flexDirection: "row", width: 20, height: 3 }}><box style={{ flexGrow: 1, height: 3 }}><text>G</text></box></box>, { cols: 20, rows: 3 });
      expect(findLayoutBySize(result.getAllLayouts(), 20, 3)).toBeDefined();
    });

    test("two equal flexGrow split evenly", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 20, height: 3 }}>
          <box style={{ flexGrow: 1, height: 3 }}><text>A</text></box>
          <box style={{ flexGrow: 1, height: 3 }}><text>B</text></box>
        </box>, { cols: 20, rows: 3 });
      expect(findAllLayoutsBySize(result.getAllLayouts(), 10, 3).length).toBeGreaterThanOrEqual(2);
    });

    test("fixed + flexGrow child", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 20, height: 3 }}>
          <box style={{ width: 5, height: 3 }}><text>F</text></box>
          <box style={{ flexGrow: 1, height: 3 }}><text>G</text></box>
        </box>, { cols: 20, rows: 3 });
      expect(findLayoutBySize(result.getAllLayouts(), 5, 3)).toBeDefined();
      expect(findLayoutBySize(result.getAllLayouts(), 15, 3)).toBeDefined();
    });

    test("flexGrow in column direction", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 10 }}>
          <box style={{ flexGrow: 1, width: 20 }}><text>A</text></box>
          <box style={{ flexGrow: 1, width: 20 }}><text>B</text></box>
        </box>, { cols: 20, rows: 10 });
      expect(findAllLayoutsBySize(result.getAllLayouts(), 20, 5).length).toBeGreaterThanOrEqual(2);
    });
  });

  describe("padding", () => {
    test("uniform padding pushes text to (2,2)", async () => {
      result = await render(<box style={{ width: 20, height: 5, padding: 2 }}><text>pad</text></box>, { cols: 20, rows: 5 });
      const pos = result.screen.findText("pad");
      expect(pos!.row).toBe(2);
      expect(pos!.col).toBe(2);
    });

    test("paddingLeft pushes text right", async () => {
      result = await render(<box style={{ width: 20, height: 3, paddingLeft: 5 }}><text>PL</text></box>, { cols: 20, rows: 3 });
      expect(result.screen.findText("PL")!.col).toBeGreaterThanOrEqual(5);
    });

    test("paddingTop pushes text down", async () => {
      result = await render(<box style={{ width: 20, height: 5, paddingTop: 3 }}><text>PT</text></box>, { cols: 20, rows: 5 });
      expect(result.screen.findText("PT")!.row).toBeGreaterThanOrEqual(3);
    });

    test("padding [vertical, horizontal]", async () => {
      result = await render(<box style={{ width: 20, height: 6, padding: [1, 3] }}><text>VH</text></box>, { cols: 20, rows: 6 });
      const pos = result.screen.findText("VH");
      expect(pos!.row).toBeGreaterThanOrEqual(1);
      expect(pos!.col).toBeGreaterThanOrEqual(3);
    });

    test("padding [top, right, bottom, left]", async () => {
      result = await render(<box style={{ width: 20, height: 6, padding: [2, 1, 1, 4] }}><text>4S</text></box>, { cols: 20, rows: 6 });
      const pos = result.screen.findText("4S");
      expect(pos!.row).toBeGreaterThanOrEqual(2);
      expect(pos!.col).toBeGreaterThanOrEqual(4);
    });

    test("padding reduces available space for children", async () => {
      result = await render(
        <box style={{ width: 20, height: 6, padding: 2 }}>
          <box style={{ flexGrow: 1 }}><text>inner</text></box>
        </box>, { cols: 20, rows: 6 });
      expect(findLayoutBySize(result.getAllLayouts(), 16, 2)).toBeDefined();
    });
  });

  describe("margin", () => {
    test("marginLeft offsets child", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 20, height: 3 }}>
          <box style={{ width: 5, height: 3, marginLeft: 3 }}><text>ML</text></box>
        </box>, { cols: 20, rows: 3 });
      expect(result.screen.findText("ML")!.col).toBeGreaterThanOrEqual(3);
    });

    test("marginTop offsets child down", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 6 }}>
          <box style={{ width: 10, height: 2, marginTop: 2 }}><text>MT</text></box>
        </box>, { cols: 20, rows: 6 });
      expect(result.screen.findText("MT")!.row).toBeGreaterThanOrEqual(2);
    });

    test("margin between siblings creates space", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 30, height: 3 }}>
          <box style={{ width: 5, height: 3 }}><text>AA</text></box>
          <box style={{ width: 5, height: 3, marginLeft: 5 }}><text>BB</text></box>
        </box>, { cols: 30, rows: 3 });
      expect(result.screen.findText("BB")!.col - (result.screen.findText("AA")!.col + 2)).toBeGreaterThanOrEqual(5);
    });
  });

  describe("gap", () => {
    test("gap between row children", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 20, height: 3, gap: 2 }}>
          <box style={{ width: 5, height: 3 }}><text>AA</text></box>
          <box style={{ width: 5, height: 3 }}><text>BB</text></box>
        </box>, { cols: 20, rows: 3 });
      expect(result.screen.findText("BB")!.col).toBeGreaterThanOrEqual(7);
    });

    test("gap between column children", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 10, gap: 2 }}>
          <box style={{ height: 2 }}><text>Top</text></box>
          <box style={{ height: 2 }}><text>Bot</text></box>
        </box>, { cols: 20, rows: 10 });
      expect(result.screen.findText("Bot")!.row).toBeGreaterThanOrEqual(4);
    });
  });

  describe("justifyContent", () => {
    test("center centers children", async () => {
      result = await render(
        <box style={{ flexDirection: "row", justifyContent: "center", width: 20, height: 3 }}>
          <box style={{ width: 4, height: 3 }}><text>CC</text></box>
        </box>, { cols: 20, rows: 3 });
      const pos = result.screen.findText("CC");
      expect(pos!.col).toBeGreaterThanOrEqual(7);
      expect(pos!.col).toBeLessThanOrEqual(9);
    });

    test("end pushes to end", async () => {
      result = await render(
        <box style={{ flexDirection: "row", justifyContent: "end", width: 20, height: 3 }}>
          <box style={{ width: 4, height: 3 }}><text>EE</text></box>
        </box>, { cols: 20, rows: 3 });
      expect(result.screen.findText("EE")!.col).toBeGreaterThanOrEqual(14);
    });

    test("start is left-aligned", async () => {
      result = await render(
        <box style={{ flexDirection: "row", justifyContent: "start", width: 20, height: 3 }}>
          <box style={{ width: 4, height: 3 }}><text>SS</text></box>
        </box>, { cols: 20, rows: 3 });
      expect(result.screen.findText("SS")!.col).toBe(0);
    });

    test("space-between distributes children", async () => {
      result = await render(
        <box style={{ flexDirection: "row", justifyContent: "space-between", width: 20, height: 3 }}>
          <box style={{ width: 2, height: 3 }}><text>A</text></box>
          <box style={{ width: 2, height: 3 }}><text>B</text></box>
        </box>, { cols: 20, rows: 3 });
      expect(result.screen.findText("A")!.col).toBe(0);
      expect(result.screen.findText("B")!.col).toBeGreaterThanOrEqual(16);
    });

    test("space-around adds space around children", async () => {
      result = await render(
        <box style={{ flexDirection: "row", justifyContent: "space-around", width: 20, height: 3 }}>
          <box style={{ width: 2, height: 3 }}><text>A</text></box>
          <box style={{ width: 2, height: 3 }}><text>B</text></box>
        </box>, { cols: 20, rows: 3 });
      expect(result.screen.findText("A")!.col).toBeGreaterThan(0);
    });
  });

  describe("alignItems", () => {
    test("center aligns cross-axis", async () => {
      result = await render(
        <box style={{ flexDirection: "row", alignItems: "center", width: 20, height: 6 }}>
          <box style={{ width: 4, height: 2 }}><text>AC</text></box>
        </box>, { cols: 20, rows: 6 });
      const pos = result.screen.findText("AC");
      expect(pos!.row).toBeGreaterThanOrEqual(1);
      expect(pos!.row).toBeLessThanOrEqual(3);
    });

    test("end aligns to bottom", async () => {
      result = await render(
        <box style={{ flexDirection: "row", alignItems: "end", width: 20, height: 6 }}>
          <box style={{ width: 4, height: 2 }}><text>AE</text></box>
        </box>, { cols: 20, rows: 6 });
      expect(result.screen.findText("AE")!.row).toBeGreaterThanOrEqual(3);
    });

    test("stretch fills cross-axis", async () => {
      result = await render(
        <box style={{ flexDirection: "row", alignItems: "stretch", width: 20, height: 6 }}>
          <box style={{ width: 10 }}><text>ST</text></box>
        </box>, { cols: 20, rows: 6 });
      expect(findLayoutBySize(result.getAllLayouts(), 10, 6)).toBeDefined();
    });
  });

  describe("flexBasis", () => {
    test("flexBasis sets initial size in row", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 20, height: 3 }}>
          <box style={{ flexBasis: 10, height: 3 }}><text>FB</text></box>
        </box>, { cols: 20, rows: 3 });
      expect(findLayoutBySize(result.getAllLayouts(), 10, 3)).toBeDefined();
    });
  });

  describe("min/max constraints", () => {
    test("maxWidth prevents growing beyond maximum", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 30, height: 3 }}>
          <box style={{ flexGrow: 1, maxWidth: 10, height: 3 }}><text>MX</text></box>
        </box>, { cols: 30, rows: 3 });
      expect(findLayoutBySize(result.getAllLayouts(), 10, 3)).toBeDefined();
    });

    test("maxHeight prevents growing beyond maximum", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 10 }}>
          <box style={{ flexGrow: 1, maxHeight: 5, width: 20 }}><text>MXH</text></box>
        </box>, { cols: 20, rows: 10 });
      expect(findLayoutBySize(result.getAllLayouts(), 20, 5)).toBeDefined();
    });
  });

  describe("nested flex", () => {
    test("row inside column", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 6 }}>
          <box style={{ flexDirection: "row", height: 3 }}><text>AA</text><text>BB</text></box>
        </box>, { cols: 20, rows: 6 });
      expect(result.screen.findText("BB")!.col).toBeGreaterThan(result.screen.findText("AA")!.col);
    });

    test("deeply nested 10 levels", async () => {
      result = await render(
        <box style={{ width: 30, height: 10 }}>
          <box><box><box><box><box><box><box><box><box><text>L10</text></box></box></box></box></box></box></box></box></box>
        </box>, { cols: 30, rows: 10 });
      expect(result.screen).toContainText("L10");
    });
  });

  describe("many children", () => {
    test("10 children in column", async () => {
      const children = Array.from({ length: 10 }, (_, i) => <box key={i} style={{ height: 1 }}><text>{`R${i}`}</text></box>);
      result = await render(<box style={{ flexDirection: "column", width: 20, height: 10 }}>{children}</box>, { cols: 20, rows: 10 });
      expect(result.screen).toContainText("R0");
      expect(result.screen).toContainText("R9");
    });

    test("20 children in column", async () => {
      const children = Array.from({ length: 20 }, (_, i) => <box key={i} style={{ height: 1 }}><text>{`X${i}`}</text></box>);
      result = await render(<box style={{ flexDirection: "column", width: 20, height: 20 }}>{children}</box>, { cols: 20, rows: 20 });
      expect(result.screen).toContainText("X0");
      expect(result.screen).toContainText("X19");
    });
  });

  describe("position validation", () => {
    test("child at correct x,y in row", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 20, height: 3 }}>
          <box style={{ width: 5, height: 3 }}><text>A</text></box>
          <box style={{ width: 5, height: 3 }}><text>B</text></box>
        </box>, { cols: 20, rows: 3 });
      expect(result.screen.findText("A")!.col).toBe(0);
      expect(result.screen.findText("B")!.col).toBe(5);
    });

    test("text after padding starts at correct position", async () => {
      result = await render(
        <box style={{ width: 20, height: 5, paddingLeft: 3, paddingTop: 1 }}><text>Here</text></box>,
        { cols: 20, rows: 5 });
      expect(result.screen.findText("Here")!.col).toBe(3);
      expect(result.screen.findText("Here")!.row).toBe(1);
    });
  });

  describe("root dimensions", () => {
    test("root matches 30x8 viewport", async () => {
      result = await render(<text>R</text>, { cols: 30, rows: 8 });
      expect(result.getLayout(1).width).toBeCloseTo(30, 0);
      expect(result.getLayout(1).height).toBeCloseTo(8, 0);
    });

    test("root matches 80x24 viewport", async () => {
      result = await render(<text>R</text>, { cols: 80, rows: 24 });
      expect(result.getLayout(1).width).toBeCloseTo(80, 0);
      expect(result.getLayout(1).height).toBeCloseTo(24, 0);
    });
  });

  describe("dimension update", () => {
    test("resize box on rerender", async () => {
      result = await render(<box style={{ width: 10, height: 3 }}><text>A</text></box>, { cols: 40, rows: 10 });
      expect(findLayoutBySize(result.getAllLayouts(), 10, 3)).toBeDefined();
      await result.rerender(<box style={{ width: 25, height: 7 }}><text>A</text></box>);
      expect(findLayoutBySize(result.getAllLayouts(), 25, 7)).toBeDefined();
    });

    test("rerender preserves text after change", async () => {
      result = await render(<box style={{ width: 20, height: 3 }}><text>Before</text></box>, { cols: 20, rows: 3 });
      expect(result.screen).toContainText("Before");
      const screen2 = await result.rerender(<box style={{ width: 20, height: 3 }}><text>After</text></box>);
      expect(screen2).toContainText("After");
    });
  });
});
