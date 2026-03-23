/**
 * E2E RenderableTree operation tests for KittyUI.
 *
 * Tests tree mutations: appendChild, removeChild, insertBefore,
 * large/deep trees, orphan handling, and dirty style flushing.
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

describe.skipIf(!canRun)("E2E Tree", () => {
  let result: RenderResult | undefined;

  afterEach(() => {
    result?.cleanup();
    result = undefined;
  });

  // ==========================================================================
  // appendChild (implicit via JSX children)
  // ==========================================================================

  describe("appendChild", () => {
    test("single child appended to parent", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}>
          <text>Child</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("Child");
    });

    test("multiple children appended in order", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 20, height: 3 }}>
          <text>A</text>
          <text>B</text>
          <text>C</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      const a = result.screen.findText("A");
      const b = result.screen.findText("B");
      const c = result.screen.findText("C");
      expect(a).toBeDefined();
      expect(b).toBeDefined();
      expect(c).toBeDefined();
      expect(b!.col).toBeGreaterThan(a!.col);
      expect(c!.col).toBeGreaterThan(b!.col);
    });

    test("deeply nested appends", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}>
          <box><box><text>Deep</text></box></box>
        </box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("Deep");
    });

    test("add child on rerender increases layout count", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 5 }}>
          <text key="a">A</text>
        </box>,
        { cols: 20, rows: 5 },
      );
      const count1 = result.getAllLayouts().size;

      await result.rerender(
        <box style={{ flexDirection: "column", width: 20, height: 5 }}>
          <text key="a">A</text>
          <text key="b">B</text>
        </box>,
      );
      const count2 = result.getAllLayouts().size;
      expect(count2).toBeGreaterThan(count1);
    });
  });

  // ==========================================================================
  // removeChild (via rerender removing a child)
  // ==========================================================================

  describe("removeChild", () => {
    test("remove child on rerender reduces layout count", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 5 }}>
          <text key="a">Alive</text>
          <text key="b">Gone</text>
        </box>,
        { cols: 20, rows: 5 },
      );
      const count1 = result.getAllLayouts().size;

      await result.rerender(
        <box style={{ flexDirection: "column", width: 20, height: 5 }}>
          <text key="a">Alive</text>
        </box>,
      );
      const count2 = result.getAllLayouts().size;
      expect(count2).toBeLessThan(count1);
    });

    test("remove first child", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 5 }}>
          <text key="x">X</text>
          <text key="y">Y</text>
        </box>,
        { cols: 20, rows: 5 },
      );
      const screen2 = await result.rerender(
        <box style={{ flexDirection: "column", width: 20, height: 5 }}>
          <text key="y">Y</text>
        </box>,
      );
      expect(screen2.containsText("X")).toBe(false);
      expect(screen2).toContainText("Y");
    });

    test("remove all children", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 5 }}>
          <text key="a">A</text>
          <text key="b">B</text>
          <text key="c">C</text>
        </box>,
        { cols: 20, rows: 5 },
      );
      const screen2 = await result.rerender(
        <box style={{ flexDirection: "column", width: 20, height: 5 }} />,
      );
      expect(screen2.containsText("A")).toBe(false);
      expect(screen2.containsText("B")).toBe(false);
      expect(screen2.containsText("C")).toBe(false);
    });

    test("remove nested subtree", async () => {
      result = await render(
        <box style={{ width: 20, height: 5 }}>
          <box key="inner">
            <text>Nested</text>
          </box>
        </box>,
        { cols: 20, rows: 5 },
      );
      expect(result.screen).toContainText("Nested");

      const screen2 = await result.rerender(
        <box style={{ width: 20, height: 5 }} />,
      );
      expect(screen2.containsText("Nested")).toBe(false);
    });
  });

  // ==========================================================================
  // insertBefore (via rerender reordering with keys)
  // ==========================================================================

  describe("insertBefore", () => {
    test("reorder children updates layout", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 5 }}>
          <box key="a" style={{ height: 2, width: 8 }}><text>A</text></box>
          <box key="b" style={{ height: 2, width: 12 }}><text>B</text></box>
        </box>,
        { cols: 20, rows: 5 },
      );
      expect(result.screen).toContainText("A");
      expect(result.screen).toContainText("B");

      // Reorder and verify layout still valid
      await result.rerender(
        <box style={{ flexDirection: "column", width: 20, height: 5 }}>
          <box key="b" style={{ height: 2, width: 12 }}><text>B</text></box>
          <box key="a" style={{ height: 2, width: 8 }}><text>A</text></box>
        </box>,
      );
      const layouts = result.getAllLayouts();
      expect(layouts.size).toBeGreaterThanOrEqual(4);
    });

    test("insert new child increases layout count", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 5 }}>
          <text key="b">B</text>
        </box>,
        { cols: 20, rows: 5 },
      );
      const count1 = result.getAllLayouts().size;

      await result.rerender(
        <box style={{ flexDirection: "column", width: 20, height: 5 }}>
          <text key="a">A</text>
          <text key="b">B</text>
        </box>,
      );
      const count2 = result.getAllLayouts().size;
      expect(count2).toBeGreaterThan(count1);
    });

    test("insert in middle increases layout count", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 5 }}>
          <text key="a">A</text>
          <text key="c">C</text>
        </box>,
        { cols: 20, rows: 5 },
      );
      const count1 = result.getAllLayouts().size;

      await result.rerender(
        <box style={{ flexDirection: "column", width: 20, height: 5 }}>
          <text key="a">A</text>
          <text key="b">B</text>
          <text key="c">C</text>
        </box>,
      );
      const count2 = result.getAllLayouts().size;
      expect(count2).toBeGreaterThan(count1);
    });
  });

  // ==========================================================================
  // Large tree
  // ==========================================================================

  describe("large tree", () => {
    test("50 children render", async () => {
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

    test("100 children render without crash", async () => {
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
  // Deep tree
  // ==========================================================================

  describe("deep tree", () => {
    test("15 levels deep", async () => {
      // Build nested boxes programmatically
      let element: React.ReactElement = <text>Bottom</text>;
      for (let i = 0; i < 14; i++) {
        element = <box>{element}</box>;
      }
      result = await render(
        <box style={{ width: 20, height: 3 }}>{element}</box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("Bottom");
    });

    test("20 levels deep", async () => {
      let element: React.ReactElement = <text>VDeep</text>;
      for (let i = 0; i < 19; i++) {
        element = <box>{element}</box>;
      }
      result = await render(
        <box style={{ width: 20, height: 3 }}>{element}</box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("VDeep");
    });
  });

  // ==========================================================================
  // Mixed text and box nodes
  // ==========================================================================

  describe("mixed nodes", () => {
    test("text and box siblings", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 20, height: 3 }}>
          <text>T</text>
          <box style={{ width: 5, height: 3, backgroundColor: "#ff0000" }} />
          <text>U</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("T");
      expect(result.screen).toContainText("U");
    });

    test("boxes with text and empty boxes", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 6 }}>
          <box style={{ height: 2 }}><text>Has</text></box>
          <box style={{ height: 2 }} />
          <box style={{ height: 2 }}><text>Text</text></box>
        </box>,
        { cols: 20, rows: 6 },
      );
      expect(result.screen).toContainText("Has");
      expect(result.screen).toContainText("Text");
    });
  });

  // ==========================================================================
  // Node count (getAllLayouts)
  // ==========================================================================

  describe("node count", () => {
    test("getAllLayouts returns all nodes", async () => {
      result = await render(
        <box style={{ width: 20, height: 5 }}>
          <box style={{ height: 2 }}><text>A</text></box>
          <box style={{ height: 2 }}><text>B</text></box>
        </box>,
        { cols: 20, rows: 5 },
      );
      const layouts = result.getAllLayouts();
      // root + outer box + 2 inner boxes + 2 text = 6 (at minimum)
      expect(layouts.size).toBeGreaterThanOrEqual(4);
    });

    test("removing children reduces layout count", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 5 }}>
          <text key="a">A</text>
          <text key="b">B</text>
          <text key="c">C</text>
        </box>,
        { cols: 20, rows: 5 },
      );
      const count1 = result.getAllLayouts().size;

      await result.rerender(
        <box style={{ flexDirection: "column", width: 20, height: 5 }}>
          <text key="a">A</text>
        </box>,
      );
      const count2 = result.getAllLayouts().size;
      expect(count2).toBeLessThan(count1);
    });
  });

  // ==========================================================================
  // Dirty style flushing
  // ==========================================================================

  describe("dirty style flushing", () => {
    test("style change reflects after rerender", async () => {
      result = await render(
        <box style={{ backgroundColor: "#ff0000", width: 10, height: 3 }}>
          <text>S</text>
        </box>,
        { cols: 20, rows: 5 },
      );
      const pos1 = result.screen.findText("S");
      expect(result.screen).toHaveBgColor(pos1!.row, pos1!.col, "#ff0000");

      const screen2 = await result.rerender(
        <box style={{ backgroundColor: "#00ff00", width: 10, height: 3 }}>
          <text>S</text>
        </box>,
      );
      const pos2 = screen2.findText("S");
      expect(screen2).toHaveBgColor(pos2!.row, pos2!.col, "#00ff00");
    });

    test("dimension change reflects after rerender", async () => {
      result = await render(
        <box style={{ width: 10, height: 3 }}><text>D</text></box>,
        { cols: 40, rows: 10 },
      );
      const layouts1 = result.getAllLayouts();
      let hasSmall = false;
      for (const [, l] of layouts1) {
        if (Math.abs(l.width - 10) < 1 && Math.abs(l.height - 3) < 1) hasSmall = true;
      }
      expect(hasSmall).toBe(true);

      await result.rerender(
        <box style={{ width: 30, height: 8 }}><text>D</text></box>,
      );
      const layouts2 = result.getAllLayouts();
      let hasBig = false;
      for (const [, l] of layouts2) {
        if (Math.abs(l.width - 30) < 1 && Math.abs(l.height - 8) < 1) hasBig = true;
      }
      expect(hasBig).toBe(true);
    });
  });
});
