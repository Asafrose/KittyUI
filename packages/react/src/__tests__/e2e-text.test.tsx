/**
 * E2E text rendering tests for KittyUI.
 *
 * Tests text content, positioning, truncation, update on rerender,
 * colors, bold/italic, unicode, and special characters.
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

describe.skipIf(!canRun)("E2E Text", () => {
  let result: RenderResult | undefined;

  afterEach(() => {
    result?.cleanup();
    result = undefined;
  });

  // ==========================================================================
  // Simple text in a box
  // ==========================================================================

  describe("simple text", () => {
    test("renders single character", async () => {
      result = await render(
        <box style={{ width: 10, height: 3 }}><text>X</text></box>,
        { cols: 10, rows: 3 },
      );
      expect(result.screen).toContainText("X");
    });

    test("renders word", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}><text>Hello</text></box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("Hello");
    });

    test("renders sentence", async () => {
      result = await render(
        <box style={{ width: 40, height: 3 }}><text>The quick brown fox</text></box>,
        { cols: 40, rows: 3 },
      );
      expect(result.screen).toContainText("The quick brown fox");
    });

    test("renders empty text without crash", async () => {
      result = await render(
        <box style={{ width: 10, height: 3 }}><text>{""}</text></box>,
        { cols: 10, rows: 3 },
      );
      expect(result.screen).toBeDefined();
    });

    test("renders text with only spaces", async () => {
      result = await render(
        <box style={{ width: 10, height: 3 }}><text>{"   "}</text></box>,
        { cols: 10, rows: 3 },
      );
      // Should not crash; spaces are valid
      expect(result.screen).toBeDefined();
    });

    test("renders two-character text", async () => {
      result = await render(
        <box style={{ width: 10, height: 3 }}><text>AB</text></box>,
        { cols: 10, rows: 3 },
      );
      expect(result.screen).toContainText("AB");
    });

    test("renders long text", async () => {
      const longText = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
      result = await render(
        <box style={{ width: 30, height: 3 }}><text>{longText}</text></box>,
        { cols: 30, rows: 3 },
      );
      expect(result.screen).toContainText("ABCDEFGHIJ");
    });
  });

  // ==========================================================================
  // Text update (rerender)
  // ==========================================================================

  describe("text update", () => {
    test("text update changes layout width", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}><text>Hi</text></box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("Hi");

      // Rerender with longer text -- layout should update
      await result.rerender(
        <box style={{ width: 20, height: 3 }}><text>LongerText</text></box>,
      );
      const layouts = result.getAllLayouts();
      let found = false;
      for (const [, l] of layouts) {
        if (l.width >= 10) found = true;
      }
      expect(found).toBe(true);
    });

    test("removing text child clears content", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}><text>Gone</text></box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("Gone");

      const screen2 = await result.rerender(
        <box style={{ width: 20, height: 3 }} />,
      );
      expect(screen2.containsText("Gone")).toBe(false);
    });
  });

  // ==========================================================================
  // Multiple text siblings
  // ==========================================================================

  describe("multiple text siblings", () => {
    test("two text nodes in row", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 20, height: 3 }}>
          <text>Hello</text>
          <text>World</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("Hello");
      expect(result.screen).toContainText("World");
      const h = result.screen.findText("Hello");
      const w = result.screen.findText("World");
      expect(w!.col).toBeGreaterThan(h!.col);
    });

    test("three text nodes in row", async () => {
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
      expect(b!.col).toBeGreaterThan(a!.col);
      expect(c!.col).toBeGreaterThan(b!.col);
    });

    test("text nodes in column", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 5 }}>
          <text>Line1</text>
          <text>Line2</text>
        </box>,
        { cols: 20, rows: 5 },
      );
      const l1 = result.screen.findText("Line1");
      const l2 = result.screen.findText("Line2");
      expect(l2!.row).toBeGreaterThan(l1!.row);
    });
  });

  // ==========================================================================
  // Text with color
  // ==========================================================================

  describe("text with color", () => {
    test("red text", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}>
          <text style={{ color: "#ff0000" }}>Red</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      const pos = result.screen.findText("Red");
      expect(result.screen).toHaveFgColor(pos!.row, pos!.col, "#ff0000");
    });

    test("green text", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}>
          <text style={{ color: "#00ff00" }}>Grn</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      const pos = result.screen.findText("Grn");
      expect(result.screen).toHaveFgColor(pos!.row, pos!.col, "#00ff00");
    });

    test("text fg color on bg", async () => {
      result = await render(
        <box style={{ backgroundColor: "#000000", width: 20, height: 3 }}>
          <text style={{ color: "#ffffff" }}>Contrast</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      const pos = result.screen.findText("Contrast");
      expect(result.screen).toHaveFgColor(pos!.row, pos!.col, "#ffffff");
      expect(result.screen).toHaveBgColor(pos!.row, pos!.col, "#000000");
    });
  });

  // ==========================================================================
  // Text with bold / italic
  // ==========================================================================

  describe("text with bold/italic", () => {
    test("bold text renders", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}>
          <text style={{ fontWeight: "bold" }}>Bold</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("Bold");
    });

    test("italic text renders", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}>
          <text style={{ fontStyle: "italic" }}>Ital</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("Ital");
    });

    test("bold + italic text renders", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}>
          <text style={{ fontWeight: "bold", fontStyle: "italic" }}>BI</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("BI");
    });
  });

  // ==========================================================================
  // Text position after padding
  // ==========================================================================

  describe("text position with spacers", () => {
    test("spacer box shifts text right", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 20, height: 3 }}>
          <box style={{ width: 5, height: 3 }} />
          <text>PL</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      const pos = result.screen.findText("PL");
      expect(pos).toBeDefined();
      expect(pos!.col).toBe(5);
    });

    test("spacer box shifts text down", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 5 }}>
          <box style={{ height: 2 }} />
          <text>PT</text>
        </box>,
        { cols: 20, rows: 5 },
      );
      const pos = result.screen.findText("PT");
      expect(pos).toBeDefined();
      expect(pos!.row).toBe(2);
    });

    test("spacers in both directions", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 5 }}>
          <box style={{ height: 2 }} />
          <box style={{ flexDirection: "row" }}>
            <box style={{ width: 3, height: 1 }} />
            <text>Both</text>
          </box>
        </box>,
        { cols: 20, rows: 5 },
      );
      const pos = result.screen.findText("Both");
      expect(pos).toBeDefined();
      expect(pos!.row).toBe(2);
      expect(pos!.col).toBe(3);
    });
  });

  // ==========================================================================
  // Text in nested boxes
  // ==========================================================================

  describe("text in nested boxes", () => {
    test("text in double-nested box", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}>
          <box><text>Nested</text></box>
        </box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("Nested");
    });

    test("text in triple-nested box", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}>
          <box><box><text>Triple</text></box></box>
        </box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("Triple");
    });

    test("text in nested box with offset", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 20, height: 5 }}>
          <box style={{ width: 5, height: 5 }} />
          <box>
            <text>DP</text>
          </box>
        </box>,
        { cols: 20, rows: 5 },
      );
      const pos = result.screen.findText("DP");
      expect(pos).toBeDefined();
      expect(pos!.col).toBeGreaterThanOrEqual(5);
    });
  });

  // ==========================================================================
  // Number as text content
  // ==========================================================================

  describe("number as text", () => {
    test("integer renders", async () => {
      result = await render(
        <box style={{ width: 10, height: 3 }}><text>42</text></box>,
        { cols: 10, rows: 3 },
      );
      expect(result.screen).toContainText("42");
    });

    test("zero renders", async () => {
      result = await render(
        <box style={{ width: 10, height: 3 }}><text>0</text></box>,
        { cols: 10, rows: 3 },
      );
      expect(result.screen).toContainText("0");
    });

    test("negative number renders", async () => {
      result = await render(
        <box style={{ width: 10, height: 3 }}><text>-1</text></box>,
        { cols: 10, rows: 3 },
      );
      expect(result.screen).toContainText("-1");
    });

    test("decimal renders", async () => {
      result = await render(
        <box style={{ width: 10, height: 3 }}><text>3.14</text></box>,
        { cols: 10, rows: 3 },
      );
      expect(result.screen).toContainText("3.14");
    });
  });

  // ==========================================================================
  // Special characters
  // ==========================================================================

  describe("special characters", () => {
    test("ampersand", async () => {
      result = await render(
        <box style={{ width: 10, height: 3 }}><text>A&B</text></box>,
        { cols: 10, rows: 3 },
      );
      expect(result.screen).toContainText("A&B");
    });

    test("less than / greater than", async () => {
      result = await render(
        <box style={{ width: 10, height: 3 }}><text>{"<>"}</text></box>,
        { cols: 10, rows: 3 },
      );
      expect(result.screen).toContainText("<>");
    });

    test("quotes", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}><text>{'"quoted"'}</text></box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText('"quoted"');
    });

    test("backslash", async () => {
      result = await render(
        <box style={{ width: 10, height: 3 }}><text>{"a\\b"}</text></box>,
        { cols: 10, rows: 3 },
      );
      expect(result.screen).toContainText("a\\b");
    });

    test("at and hash signs", async () => {
      result = await render(
        <box style={{ width: 10, height: 3 }}><text>@#$%</text></box>,
        { cols: 10, rows: 3 },
      );
      expect(result.screen).toContainText("@#$%");
    });

    test("parentheses and brackets", async () => {
      result = await render(
        <box style={{ width: 10, height: 3 }}><text>()[]</text></box>,
        { cols: 10, rows: 3 },
      );
      expect(result.screen).toContainText("()[]");
    });
  });

  // ==========================================================================
  // Text at specific row/col
  // ==========================================================================

  describe("text position verification", () => {
    test("text at row 0, col 0", async () => {
      result = await render(
        <box style={{ width: 10, height: 3 }}><text>Hello</text></box>,
        { cols: 10, rows: 3 },
      );
      expect(result.screen).toHaveTextAt(0, 0, "Hello");
    });

    test("text at row 3 after spacer", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 6 }}>
          <box style={{ height: 3 }} />
          <text>Below</text>
        </box>,
        { cols: 20, rows: 6 },
      );
      expect(result.screen).toHaveTextAt(3, 0, "Below");
    });

    test("text at col 5 after left box", async () => {
      result = await render(
        <box style={{ flexDirection: "row", width: 20, height: 3 }}>
          <box style={{ width: 5, height: 3 }} />
          <text>Right</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toHaveTextAt(0, 5, "Right");
    });
  });
});
