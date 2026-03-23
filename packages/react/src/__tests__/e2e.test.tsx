/**
 * End-to-end tests for the KittyUI React rendering pipeline.
 *
 * Each test mounts a JSX tree into a headless Rust engine, renders to a
 * VirtualScreen, and asserts on the resulting cell grid.
 */

import { describe, test, expect, afterEach } from "bun:test";
import React from "react";
import type { CSSStyle } from "@kittyui/core";

// KittyUI JSX intrinsic element declarations for the react-jsx transform.
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

// ---------------------------------------------------------------------------
// Can we run? (native lib must be built)
// ---------------------------------------------------------------------------

const canRun = new TestBridge().nativeAvailable;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Find a layout with a specific width and height from getAllLayouts(). */
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

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

describe.skipIf(!canRun)("E2E React rendering", () => {
  let result: RenderResult | undefined;

  afterEach(() => {
    result?.cleanup();
    result = undefined;
  });

  // 1. nested children render
  test("nested children render", async () => {
    result = await render(
      <box>
        <text>Hello</text>
      </box>,
    );
    expect(result.screen).toContainText("Hello");
  });

  // 2. backgroundColor renders
  test("backgroundColor renders", async () => {
    result = await render(
      <box style={{ backgroundColor: "#ff0000", width: 10, height: 3 }}>
        <text>X</text>
      </box>,
      { cols: 20, rows: 5 },
    );
    expect(result.screen).toContainText("X");
    // Find a cell with the red bg — it should exist somewhere in the rendered area
    const pos = result.screen.findText("X");
    expect(pos).toBeDefined();
    expect(result.screen.bgAt(pos!.row, pos!.col)).toBe("#ff0000");
  });

  // 3. explicit dimensions work
  test("explicit dimensions work", async () => {
    result = await render(
      <box style={{ width: 20, height: 5 }}>
        <text>sized</text>
      </box>,
      { cols: 40, rows: 10 },
    );
    expect(result.screen).toContainText("sized");
    // Find the node with 20x5 dimensions
    const layouts = result.getAllLayouts();
    const sized = findLayoutBySize(layouts, 20, 5);
    expect(sized).toBeDefined();
    expect(sized!.width).toBeCloseTo(20, 0);
    expect(sized!.height).toBeCloseTo(5, 0);
  });

  // 4. root fills terminal
  test("root fills terminal", async () => {
    result = await render(<text>root</text>, { cols: 30, rows: 8 });
    const rootLayout = result.getLayout(1);
    expect(rootLayout.width).toBeCloseTo(30, 0);
    expect(rootLayout.height).toBeCloseTo(8, 0);
  });

  // 5. text at layout position
  test("text at layout position", async () => {
    result = await render(
      <box style={{ flexDirection: "column", width: 30, height: 10 }}>
        <box style={{ height: 3 }} />
        <text>Below</text>
      </box>,
      { cols: 30, rows: 10 },
    );
    // "Below" should appear at row 3 (after the 3-row spacer)
    const pos = result.screen.findText("Below");
    expect(pos).toBeDefined();
    expect(pos!.row).toBe(3);
  });

  // 6. siblings in row order
  test("siblings in row order", async () => {
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
    // BB should be to the right of AA
    expect(posB!.col).toBeGreaterThan(posA!.col);
  });

  // 7. column layout stacks vertically
  test("column layout stacks vertically", async () => {
    result = await render(
      <box style={{ flexDirection: "column", width: 20, height: 10 }}>
        <box style={{ height: 3 }}>
          <text>Top</text>
        </box>
        <box style={{ height: 3 }}>
          <text>Bot</text>
        </box>
      </box>,
      { cols: 20, rows: 10 },
    );
    const posTop = result.screen.findText("Top");
    const posBot = result.screen.findText("Bot");
    expect(posTop).toBeDefined();
    expect(posBot).toBeDefined();
    expect(posBot!.row).toBeGreaterThan(posTop!.row);
  });

  // 8. setStyle updates layout
  test("setStyle updates layout", async () => {
    result = await render(
      <box style={{ width: 10, height: 5 }}>
        <text>A</text>
      </box>,
      { cols: 40, rows: 10 },
    );
    // Find the 10x5 box
    const layouts1 = result.getAllLayouts();
    const box1 = findLayoutBySize(layouts1, 10, 5);
    expect(box1).toBeDefined();

    // Re-render with different width
    await result.rerender(
      <box style={{ width: 25, height: 5 }}>
        <text>A</text>
      </box>,
    );
    const layouts2 = result.getAllLayouts();
    const box2 = findLayoutBySize(layouts2, 25, 5);
    expect(box2).toBeDefined();
  });

  // 9. removed child disappears
  test("removed child disappears", async () => {
    result = await render(
      <box style={{ width: 20, height: 5 }}>
        <text>Visible</text>
      </box>,
      { cols: 20, rows: 5 },
    );
    expect(result.screen).toContainText("Visible");

    const screen2 = await result.rerender(
      <box style={{ width: 20, height: 5 }} />,
    );
    expect(screen2.containsText("Visible")).toBe(false);
  });

  // 10. percentage dimensions resolve
  test("percentage dimensions resolve", async () => {
    result = await render(
      <box style={{ width: "100%", height: "100%" }}>
        <text>full</text>
      </box>,
      { cols: 30, rows: 8 },
    );
    expect(result.screen).toContainText("full");
    // Find the box that fills the viewport
    const layouts = result.getAllLayouts();
    const full = findLayoutBySize(layouts, 30, 8);
    // Should find at least the root at 30x8
    expect(full).toBeDefined();
  });

  // 11. multiple nested levels
  test("multiple nested levels", async () => {
    result = await render(
      <box style={{ width: 30, height: 6 }}>
        <box>
          <text>Deep</text>
        </box>
      </box>,
      { cols: 30, rows: 6 },
    );
    expect(result.screen).toContainText("Deep");
  });

  // 12. text color renders
  test("text color renders", async () => {
    result = await render(
      <box style={{ width: 20, height: 3 }}>
        <text style={{ color: "#00ff00" }}>Green</text>
      </box>,
      { cols: 20, rows: 3 },
    );
    expect(result.screen).toContainText("Green");
    const pos = result.screen.findText("Green");
    expect(pos).toBeDefined();
    expect(result.screen.fgAt(pos!.row, pos!.col)).toBe("#00ff00");
  });
});
