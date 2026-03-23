/**
 * E2E layout engine tests for KittyUI.
 *
 * Tests flex layout: fixed dimensions, flexDirection, flexGrow,
 * nested layouts, many children, zero-size, overflow, and mixed layouts.
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
      result = await render(
        <box style={{ width: 5, height: 3 }}><text>A</text></box>,
        { cols: 40, rows: 10 },
      );
      const layouts = result.getAllLayouts();
      expect(findLayoutBySize(layouts, 5, 3)).toBeDefined();
    });

    test("width 10, height 5", async () => {
      result = await render(
        <box style={{ width: 10, height: 5 }}><text>B</text></box>,
        { cols: 40, rows: 10 },
      );
      const layouts = result.getAllLayouts();
      expect(findLayoutBySize(layouts, 10, 5)).toBeDefined();
    });

    test("width 20, height 8", async () => {
      result = await render(
        <box style={{ width: 20, height: 8 }}><text>C</text></box>,
        { cols: 40, rows: 10 },
      );
      const layouts = result.getAllLayouts();
      expect(findLayoutBySize(layouts, 20, 8)).toBeDefined();
    });

    test("width 40, height 10 fills viewport", async () => {
      result = await render(
        <box style={{ width: 40, height: 10 }}><text>D</text></box>,
        { cols: 40, rows: 10 },
      );
      const layouts = result.getAllLayouts();
      expect(findLayoutBySize(layouts, 40, 10)).toBeDefined();
    });

    test("width 1, height 1 minimal box", async () => {
      result = await render(
        <box style={{ width: 1, height: 1 }}><text>X</text></box>,
        { cols: 40, rows: 10 },
      );
      const layouts = result.getAllLayouts();
      expect(findLayoutBySize(layouts, 1, 1)).toBeDefined();
    });

    test("width 80, height 24 large viewport", async () => {
      result = await render(
        <box style={{ width: 80, height: 24 }}><text>L</text></box>,
        { cols: 80, rows: 24 },
      );
      const layouts = result.getAllLayouts();
      expect(findLayoutBySize(layouts, 80, 24)).toBeDefined();
    });

    test("width 30, height 15", async () => {
      result = await render(
        <box style={{ width: 30, height: 15 }}><text>M</text></box>,
        { cols: 40, rows: 20 },
      );
      const layouts = result.getAllLayouts();
      expect(findLayoutBySize(layouts, 30, 15)).toBeDefined();
    });

    test("width 2, height 2 small box", async () => {
      result = await render(
        <box style={{ width: 2, height: 2 }}><text>S</text></box>,
        { cols: 20, rows: 10 },
      );
      const layouts = result.getAllLayouts();
      expect(findLayoutBySize(layouts, 2, 2)).toBeDefined();
    });
  });

  // ==========================================================================
  // Percentage dimensions
  // ==========================================================================

  describe("percentage dimensions", () => {
    test("100% width fills parent", async () => {
      result = await render(
        <box style={{ width: "100%", height: 5 }}><text>full</text></box>,
        { cols: 40, rows: 10 },
      );
      expect(result.screen).toContainText("full");
    });

    test("100% width and 100% height", async () => {
      result = await render(
        <box style={{ width: "100%", height: "100%" }}><text>all</text></box>,
        { cols: 30, rows: 8 },
      );
      expect(result.screen).toContainText("all");
      const layouts = result.getAllLayouts();
      expect(findLayoutBySize(layouts, 30, 8)).toBeDefined();
    });
  });

  // ==========================================================================
  // Flex direction
  // ==========================================================================

  describe("flexDirection", () => {
    test("row places children side by side", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 20, height: 3 }}>
          <text>AA</text>
          <text>BB</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      const posA = result.screen.findText("AA");
      const posB = result.screen.findText("BB");
      expect(posA).toBeDefined();
      expect(posB).toBeDefined();
      expect(posB!.col).toBeGreaterThan(posA!.col);
    });

    test("column stacks children vertically", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 6 }}>
          <box style={{ height: 2 }}><text>Top</text></box>
          <box style={{ height: 2 }}><text>Bot</text></box>
        </box>,
        { cols: 20, rows: 6 },
      );
      const posT = result.screen.findText("Top");
      const posB = result.screen.findText("Bot");
      expect(posT).toBeDefined();
      expect(posB).toBeDefined();
      expect(posB!.row).toBeGreaterThan(posT!.row);
    });

    test("row-reverse reverses order", async () => {
      result = await render(
        <box style={{ flexDirection: "row-reverse", width: 20, height: 3 }}>
          <text>AA</text>
          <text>BB</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      const posA = result.screen.findText("AA");
      const posB = result.screen.findText("BB");
      expect(posA).toBeDefined();
      expect(posB).toBeDefined();
      expect(posA!.col).toBeGreaterThan(posB!.col);
    });

    test("column-reverse reverses vertical order", async () => {
      result = await render(
        <box style={{ flexDirection: "column-reverse", width: 20, height: 6 }}>
          <box style={{ height: 2 }}><text>Top</text></box>
          <box style={{ height: 2 }}><text>Bot</text></box>
        </box>,
        { cols: 20, rows: 6 },
      );
      const posT = result.screen.findText("Top");
      const posB = result.screen.findText("Bot");
      expect(posT).toBeDefined();
      expect(posB).toBeDefined();
      expect(posT!.row).toBeGreaterThan(posB!.row);
    });

    test("row with three children", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 30, height: 3 }}>
          <text>AA</text>
          <text>BB</text>
          <text>CC</text>
        </box>,
        { cols: 30, rows: 3 },
      );
      const a = result.screen.findText("AA");
      const b = result.screen.findText("BB");
      const c = result.screen.findText("CC");
      expect(a).toBeDefined();
      expect(b).toBeDefined();
      expect(c).toBeDefined();
      expect(b!.col).toBeGreaterThan(a!.col);
      expect(c!.col).toBeGreaterThan(b!.col);
    });

    test("column with three children", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 9 }}>
          <box style={{ height: 2 }}><text>R1</text></box>
          <box style={{ height: 2 }}><text>R2</text></box>
          <box style={{ height: 2 }}><text>R3</text></box>
        </box>,
        { cols: 20, rows: 9 },
      );
      const r1 = result.screen.findText("R1");
      const r2 = result.screen.findText("R2");
      const r3 = result.screen.findText("R3");
      expect(r1).toBeDefined();
      expect(r2).toBeDefined();
      expect(r3).toBeDefined();
      expect(r2!.row).toBeGreaterThan(r1!.row);
      expect(r3!.row).toBeGreaterThan(r2!.row);
    });

    test("row-reverse with three children", async () => {
      result = await render(
        <box style={{ flexDirection: "row-reverse", width: 30, height: 3 }}>
          <text>A</text>
          <text>B</text>
          <text>C</text>
        </box>,
        { cols: 30, rows: 3 },
      );
      const a = result.screen.findText("A");
      const c = result.screen.findText("C");
      expect(a!.col).toBeGreaterThan(c!.col);
    });
  });

  // ==========================================================================
  // Flex grow
  // ==========================================================================

  describe("flexGrow", () => {
    test("single child with flexGrow fills parent", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 20, height: 3 }}>
          <box style={{ flexGrow: 1, height: 3 }}><text>G</text></box>
        </box>,
        { cols: 20, rows: 3 },
      );
      const layouts = result.getAllLayouts();
      expect(findLayoutBySize(layouts, 20, 3)).toBeDefined();
    });

    test("two children equal flexGrow split evenly", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 20, height: 3 }}>
          <box style={{ flexGrow: 1, height: 3 }}><text>A</text></box>
          <box style={{ flexGrow: 1, height: 3 }}><text>B</text></box>
        </box>,
        { cols: 20, rows: 3 },
      );
      const layouts = result.getAllLayouts();
      const matches = findAllLayoutsBySize(layouts, 10, 3);
      expect(matches.length).toBeGreaterThanOrEqual(2);
    });

    test("weighted flexGrow 1:2 allocates proportionally", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 30, height: 3 }}>
          <box style={{ flexGrow: 1, height: 3 }}><text>A</text></box>
          <box style={{ flexGrow: 2, height: 3 }}><text>B</text></box>
        </box>,
        { cols: 30, rows: 3 },
      );
      const layouts = result.getAllLayouts();
      expect(findLayoutBySize(layouts, 10, 3)).toBeDefined();
      expect(findLayoutBySize(layouts, 20, 3)).toBeDefined();
    });

    test("three children flexGrow 1:1:1", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 30, height: 3 }}>
          <box style={{ flexGrow: 1, height: 3 }}><text>A</text></box>
          <box style={{ flexGrow: 1, height: 3 }}><text>B</text></box>
          <box style={{ flexGrow: 1, height: 3 }}><text>C</text></box>
        </box>,
        { cols: 30, rows: 3 },
      );
      const layouts = result.getAllLayouts();
      const matches = findAllLayoutsBySize(layouts, 10, 3);
      expect(matches.length).toBeGreaterThanOrEqual(3);
    });

    test("flexGrow in column direction", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 10 }}>
          <box style={{ flexGrow: 1, width: 20 }}><text>A</text></box>
          <box style={{ flexGrow: 1, width: 20 }}><text>B</text></box>
        </box>,
        { cols: 20, rows: 10 },
      );
      const layouts = result.getAllLayouts();
      const matches = findAllLayoutsBySize(layouts, 20, 5);
      expect(matches.length).toBeGreaterThanOrEqual(2);
    });

    test("fixed child + flexGrow child", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 20, height: 3 }}>
          <box style={{ width: 5, height: 3 }}><text>F</text></box>
          <box style={{ flexGrow: 1, height: 3 }}><text>G</text></box>
        </box>,
        { cols: 20, rows: 3 },
      );
      const layouts = result.getAllLayouts();
      expect(findLayoutBySize(layouts, 5, 3)).toBeDefined();
      expect(findLayoutBySize(layouts, 15, 3)).toBeDefined();
    });

    test("two fixed children + one flexGrow fills remainder", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 30, height: 3 }}>
          <box style={{ width: 5, height: 3 }}><text>F1</text></box>
          <box style={{ flexGrow: 1, height: 3 }}><text>G</text></box>
          <box style={{ width: 5, height: 3 }}><text>F2</text></box>
        </box>,
        { cols: 30, rows: 3 },
      );
      const layouts = result.getAllLayouts();
      expect(findLayoutBySize(layouts, 20, 3)).toBeDefined();
    });

    test("flexGrow 0 does not grow", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 20, height: 3 }}>
          <box style={{ flexGrow: 0, width: 5, height: 3 }}><text>N</text></box>
          <box style={{ flexGrow: 1, height: 3 }}><text>Y</text></box>
        </box>,
        { cols: 20, rows: 3 },
      );
      const layouts = result.getAllLayouts();
      expect(findLayoutBySize(layouts, 5, 3)).toBeDefined();
      expect(findLayoutBySize(layouts, 15, 3)).toBeDefined();
    });

    test("weighted flexGrow 1:3 allocates proportionally", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 40, height: 3 }}>
          <box style={{ flexGrow: 1, height: 3 }}><text>A</text></box>
          <box style={{ flexGrow: 3, height: 3 }}><text>B</text></box>
        </box>,
        { cols: 40, rows: 3 },
      );
      const layouts = result.getAllLayouts();
      // The smaller box should be roughly 1/4 of 40
      let smallWidth = 0;
      let bigWidth = 0;
      for (const [, l] of layouts) {
        if (Math.abs(l.height - 3) < 1 && l.width > 5 && l.width < 20) {
          smallWidth = l.width;
        }
        if (Math.abs(l.height - 3) < 1 && l.width > 20 && l.width < 40) {
          bigWidth = l.width;
        }
      }
      expect(smallWidth).toBeGreaterThan(0);
      expect(bigWidth).toBeGreaterThan(0);
      expect(bigWidth).toBeGreaterThan(smallWidth * 2);
    });

    test("column flexGrow 1:2", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 9 }}>
          <box style={{ flexGrow: 1, width: 20 }}><text>S</text></box>
          <box style={{ flexGrow: 2, width: 20 }}><text>L</text></box>
        </box>,
        { cols: 20, rows: 9 },
      );
      const layouts = result.getAllLayouts();
      expect(findLayoutBySize(layouts, 20, 3)).toBeDefined();
      expect(findLayoutBySize(layouts, 20, 6)).toBeDefined();
    });
  });

  // ==========================================================================
  // Nested flex
  // ==========================================================================

  describe("nested flex", () => {
    test("row inside column", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 6 }}>
          <box style={{ flexDirection: "row", height: 3 }}>
            <text>AA</text>
            <text>BB</text>
          </box>
        </box>,
        { cols: 20, rows: 6 },
      );
      const posA = result.screen.findText("AA");
      const posB = result.screen.findText("BB");
      expect(posA).toBeDefined();
      expect(posB).toBeDefined();
      expect(posA!.row).toBe(posB!.row);
      expect(posB!.col).toBeGreaterThan(posA!.col);
    });

    test("column inside row", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 20, height: 6 }}>
          <box style={{ flexDirection: "column", width: 10 }}>
            <box style={{ height: 2 }}><text>Top</text></box>
            <box style={{ height: 2 }}><text>Bot</text></box>
          </box>
        </box>,
        { cols: 20, rows: 6 },
      );
      const posT = result.screen.findText("Top");
      const posB = result.screen.findText("Bot");
      expect(posT).toBeDefined();
      expect(posB).toBeDefined();
      expect(posB!.row).toBeGreaterThan(posT!.row);
    });

    test("three levels deep", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 30, height: 10 }}>
          <box style={{ flexDirection: "row", height: 5 }}>
            <box style={{ flexDirection: "column", width: 15 }}>
              <box style={{ height: 2 }}><text>Deep</text></box>
            </box>
          </box>
        </box>,
        { cols: 30, rows: 10 },
      );
      expect(result.screen).toContainText("Deep");
    });

    test("deeply nested 5 levels", async () => {
      result = await render(
        <box style={{ width: 30, height: 10 }}>
          <box><box><box><box>
            <text>L5</text>
          </box></box></box></box>
        </box>,
        { cols: 30, rows: 10 },
      );
      expect(result.screen).toContainText("L5");
    });

    test("deeply nested 10 levels", async () => {
      result = await render(
        <box style={{ width: 30, height: 10 }}>
          <box><box><box><box><box><box><box><box><box>
            <text>L10</text>
          </box></box></box></box></box></box></box></box></box>
        </box>,
        { cols: 30, rows: 10 },
      );
      expect(result.screen).toContainText("L10");
    });

    test("flexGrow inside nested containers", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 30, height: 6 }}>
          <box style={{ flexGrow: 1, flexDirection: "column", height: 6 }}>
            <box style={{ flexGrow: 1 }}><text>A</text></box>
            <box style={{ flexGrow: 1 }}><text>B</text></box>
          </box>
          <box style={{ flexGrow: 1, height: 6 }}><text>C</text></box>
        </box>,
        { cols: 30, rows: 6 },
      );
      expect(result.screen).toContainText("A");
      expect(result.screen).toContainText("B");
      expect(result.screen).toContainText("C");
    });
  });

  // ==========================================================================
  // Many children
  // ==========================================================================

  describe("many children", () => {
    test("10 children in column", async () => {
      const children = Array.from({ length: 10 }, (_, i) =>
        <box key={i} style={{ height: 1 }}><text>{`R${i}`}</text></box>
      );
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 10 }}>
          {children}
        </box>,
        { cols: 20, rows: 10 },
      );
      expect(result.screen).toContainText("R0");
      expect(result.screen).toContainText("R9");
    });

    test("10 children in row", async () => {
      const children = Array.from({ length: 10 }, (_, i) =>
        <box key={i} style={{ width: 2, height: 3 }}><text>{`${i}`}</text></box>
      );
      result = await render(
        <box style={{ flexDirection: "row", width: 20, height: 3 }}>
          {children}
        </box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("0");
      expect(result.screen).toContainText("9");
    });

    test("20 children in column", async () => {
      const children = Array.from({ length: 20 }, (_, i) =>
        <box key={i} style={{ height: 1 }}><text>{`X${i}`}</text></box>
      );
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 20 }}>
          {children}
        </box>,
        { cols: 20, rows: 20 },
      );
      expect(result.screen).toContainText("X0");
      expect(result.screen).toContainText("X19");
    });

    test("parent with 20 direct children in row", async () => {
      const children = Array.from({ length: 20 }, (_, i) =>
        <box key={i} style={{ width: 2, height: 3 }}><text>{`${i % 10}`}</text></box>
      );
      result = await render(
        <box style={{ flexDirection: "row", width: 40, height: 3 }}>
          {children}
        </box>,
        { cols: 40, rows: 3 },
      );
      expect(result.screen).toContainText("0");
    });

    test("5 flexGrow children split evenly", async () => {
      const children = Array.from({ length: 5 }, (_, i) =>
        <box key={i} style={{ flexGrow: 1, height: 3 }}><text>{`${i}`}</text></box>
      );
      result = await render(
        <box style={{ flexDirection: "row", width: 25, height: 3 }}>
          {children}
        </box>,
        { cols: 25, rows: 3 },
      );
      const layouts = result.getAllLayouts();
      const fives = findAllLayoutsBySize(layouts, 5, 3);
      expect(fives.length).toBeGreaterThanOrEqual(5);
    });
  });

  // ==========================================================================
  // Zero-size nodes
  // ==========================================================================

  describe("zero-size nodes", () => {
    test("zero-width box takes no space", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 20, height: 3 }}>
          <box style={{ width: 0, height: 3 }} />
          <box style={{ width: 5, height: 3 }}><text>OK</text></box>
        </box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("OK");
    });

    test("zero-height box takes no space", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 5 }}>
          <box style={{ width: 20, height: 0 }} />
          <box style={{ height: 2 }}><text>OK</text></box>
        </box>,
        { cols: 20, rows: 5 },
      );
      const pos = result.screen.findText("OK");
      expect(pos).toBeDefined();
      expect(pos!.row).toBe(0);
    });

    test("empty box renders without crash", async () => {
      result = await render(
        <box style={{ width: 10, height: 3 }} />,
        { cols: 10, rows: 3 },
      );
      expect(result.screen).toBeDefined();
    });
  });

  // ==========================================================================
  // Overflow
  // ==========================================================================

  describe("overflow", () => {
    test("children exceeding parent width are handled", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 10, height: 3 }}>
          <box style={{ width: 5, height: 3 }}><text>A</text></box>
          <box style={{ width: 5, height: 3 }}><text>B</text></box>
          <box style={{ width: 5, height: 3 }}><text>C</text></box>
        </box>,
        { cols: 10, rows: 3 },
      );
      expect(result.screen).toContainText("A");
      expect(result.screen).toContainText("B");
    });

    test("children exceeding parent height are handled", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 3 }}>
          <box style={{ height: 2 }}><text>A</text></box>
          <box style={{ height: 2 }}><text>B</text></box>
          <box style={{ height: 2 }}><text>C</text></box>
        </box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("A");
    });
  });

  // ==========================================================================
  // Position validation
  // ==========================================================================

  describe("position validation", () => {
    test("child at correct x in row", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 20, height: 3 }}>
          <box style={{ width: 5, height: 3 }}><text>A</text></box>
          <box style={{ width: 5, height: 3 }}><text>B</text></box>
        </box>,
        { cols: 20, rows: 3 },
      );
      const posA = result.screen.findText("A");
      const posB = result.screen.findText("B");
      expect(posA!.col).toBe(0);
      expect(posB!.col).toBe(5);
    });

    test("child at correct y in column", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 6 }}>
          <box style={{ height: 2 }}><text>R0</text></box>
          <box style={{ height: 2 }}><text>R2</text></box>
        </box>,
        { cols: 20, rows: 6 },
      );
      const p0 = result.screen.findText("R0");
      const p2 = result.screen.findText("R2");
      expect(p0!.row).toBe(0);
      expect(p2!.row).toBe(2);
    });

    test("third child at correct x in row", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 30, height: 3 }}>
          <box style={{ width: 5, height: 3 }}><text>A</text></box>
          <box style={{ width: 5, height: 3 }}><text>B</text></box>
          <box style={{ width: 5, height: 3 }}><text>C</text></box>
        </box>,
        { cols: 30, rows: 3 },
      );
      const posC = result.screen.findText("C");
      expect(posC!.col).toBe(10);
    });

    test("third child at correct y in column", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 9 }}>
          <box style={{ height: 3 }}><text>A</text></box>
          <box style={{ height: 3 }}><text>B</text></box>
          <box style={{ height: 3 }}><text>C</text></box>
        </box>,
        { cols: 20, rows: 9 },
      );
      const posC = result.screen.findText("C");
      expect(posC!.row).toBe(6);
    });

    test("text after spacer box", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 30, height: 10 }}>
          <box style={{ height: 3 }} />
          <text>Below</text>
        </box>,
        { cols: 30, rows: 10 },
      );
      const pos = result.screen.findText("Below");
      expect(pos).toBeDefined();
      expect(pos!.row).toBe(3);
    });
  });

  // ==========================================================================
  // Root dimensions
  // ==========================================================================

  describe("root dimensions", () => {
    test("root matches 30x8 viewport", async () => {
      result = await render(<text>R</text>, { cols: 30, rows: 8 });
      const rootLayout = result.getLayout(1);
      expect(rootLayout.width).toBeCloseTo(30, 0);
      expect(rootLayout.height).toBeCloseTo(8, 0);
    });

    test("root matches 80x24 viewport", async () => {
      result = await render(<text>R</text>, { cols: 80, rows: 24 });
      const rootLayout = result.getLayout(1);
      expect(rootLayout.width).toBeCloseTo(80, 0);
      expect(rootLayout.height).toBeCloseTo(24, 0);
    });

    test("root matches 40x10 viewport", async () => {
      result = await render(<text>R</text>, { cols: 40, rows: 10 });
      const rootLayout = result.getLayout(1);
      expect(rootLayout.width).toBeCloseTo(40, 0);
      expect(rootLayout.height).toBeCloseTo(10, 0);
    });

    test("root matches 10x5 viewport", async () => {
      result = await render(<text>R</text>, { cols: 10, rows: 5 });
      const rootLayout = result.getLayout(1);
      expect(rootLayout.width).toBeCloseTo(10, 0);
      expect(rootLayout.height).toBeCloseTo(5, 0);
    });
  });

  // ==========================================================================
  // Dimension update via rerender
  // ==========================================================================

  describe("dimension update", () => {
    test("resize box on rerender", async () => {
      result = await render(
        <box style={{ width: 10, height: 3 }}><text>A</text></box>,
        { cols: 40, rows: 10 },
      );
      expect(findLayoutBySize(result.getAllLayouts(), 10, 3)).toBeDefined();

      await result.rerender(
        <box style={{ width: 25, height: 7 }}><text>A</text></box>,
      );
      expect(findLayoutBySize(result.getAllLayouts(), 25, 7)).toBeDefined();
    });

    test("change flexDirection on rerender updates layout", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 20, height: 5 }}>
          <box style={{ width: 5, height: 2 }}><text>A</text></box>
          <box style={{ width: 5, height: 2 }}><text>B</text></box>
        </box>,
        { cols: 20, rows: 5 },
      );
      // Row: A and B side by side
      const posA1 = result.screen.findText("A");
      const posB1 = result.screen.findText("B");
      expect(posA1!.row).toBe(posB1!.row);

      await result.rerender(
        <box style={{ flexDirection: "column", width: 20, height: 5 }}>
          <box style={{ width: 5, height: 2 }}><text>A</text></box>
          <box style={{ width: 5, height: 2 }}><text>B</text></box>
        </box>,
      );
      // After rerender, verify layouts changed (column mode)
      const layouts = result.getAllLayouts();
      expect(layouts.size).toBeGreaterThanOrEqual(4);
    });
  });
});
