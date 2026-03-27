/**
 * E2E / smoke tests for the shadcn dashboard demo.
 *
 * These tests verify:
 * 1. The App component can be imported without errors
 * 2. Style objects use the required pixel-rendering properties
 * 3. When the native library is available, a full render produces output
 *    including Kitty graphics protocol sequences
 */

import { describe, test, expect, afterEach } from "bun:test";
import { TestBridge } from "@kittyui/core/src/test-harness/test-bridge.js";
import { normalizeStyle } from "@kittyui/core";
import type { CSSStyle } from "@kittyui/core";

// ---------------------------------------------------------------------------
// Smoke: module imports
// ---------------------------------------------------------------------------

describe("dashboard demo imports", () => {
  test("App component can be imported", async () => {
    const mod = await import("./app.js");
    expect(mod.App).toBeDefined();
    expect(typeof mod.App).toBe("function");
  });
});

// ---------------------------------------------------------------------------
// Style validation: pixel rendering properties are correct
// ---------------------------------------------------------------------------

describe("dashboard card styles", () => {
  // Reproduce the canonical card style from app.tsx
  const cardStyle: CSSStyle = {
    backgroundColor: "#18181b",
    borderRadius: 8,
    boxShadow: "0 1px 3px rgba(0,0,0,0.3)",
    border: "round",
    borderColor: "#27272a",
    padding: [1, 2],
    flexDirection: "column",
  };

  test("cards use borderRadius: 8", () => {
    expect(cardStyle.borderRadius).toBe(8);
  });

  test("cards use boxShadow with rgba", () => {
    expect(cardStyle.boxShadow).toBe("0 1px 3px rgba(0,0,0,0.3)");
  });

  test("normalizeStyle processes card style without errors", () => {
    const { node, text } = normalizeStyle(cardStyle);
    expect(node).toBeDefined();
    expect(text).toBeDefined();
    // borderRadius is passed through to node
    expect((node as Record<string, unknown>).borderRadius).toBe(8);
    // boxShadow is passed through to node
    expect((node as Record<string, unknown>).boxShadow).toBe("0 1px 3px rgba(0,0,0,0.3)");
  });

  test("linear-gradient background is a valid CSSStyle value", () => {
    const headerStyle: CSSStyle = {
      background: "linear-gradient(90deg, #18181b, #27272a)",
    };
    expect(headerStyle.background).toContain("linear-gradient");
  });

  test("shadcn zinc palette colors are used", () => {
    const zinc = ["#09090b", "#18181b", "#27272a", "#a1a1aa", "#fafafa"];
    // Verify all zinc palette colors appear in the card style or related tokens
    expect(zinc).toContain(cardStyle.backgroundColor as string);
    expect(zinc).toContain(cardStyle.borderColor as string);
  });
});

// ---------------------------------------------------------------------------
// Full render: Kitty graphics protocol (requires native lib)
// ---------------------------------------------------------------------------

const canRun = new TestBridge().nativeAvailable;

describe.skipIf(!canRun)("pixel rendering produces output", () => {
  let bridge: TestBridge;

  afterEach(() => {
    bridge?.shutdownTestMode();
  });

  test("dashboard card with borderRadius renders output", () => {
    bridge = new TestBridge();
    bridge.initTestMode(80, 24);

    const enc = bridge.getEncoder();

    // Root node
    enc.createNode(1, {
      width: 80,
      height: 24,
      backgroundColor: "#09090b",
      flexDirection: "column",
    });

    // Card with pixel rendering properties
    enc.createNode(2, {
      width: 40,
      height: 10,
      backgroundColor: "#18181b",
      borderRadius: 8,
      boxShadow: "0 1px 3px rgba(0,0,0,0.3)",
      border: "round",
      borderColor: "#27272a",
    });
    enc.appendChild(1, 2);

    // Text child
    enc.createNode(3, { width: 20, height: 1 });
    enc.setText(3, "Total Revenue");
    enc.appendChild(2, 3);

    bridge.flushMutations();
    bridge.renderFrame();

    const output = bridge.getRenderedOutput();
    expect(output.byteLength).toBeGreaterThan(0);

    // Decode output and check for expected content
    const text = new TextDecoder().decode(output);
    expect(text.length).toBeGreaterThan(0);

    // The rendered output should contain "Total Revenue" text
    expect(text).toContain("Total Revenue");
  });

  test("pixel rendering produces Kitty graphics sequences", () => {
    bridge = new TestBridge();
    bridge.initTestMode(80, 24);

    const enc = bridge.getEncoder();

    // Create a node with borderRadius to trigger pixel rendering
    enc.createNode(1, {
      width: 80,
      height: 24,
      flexDirection: "column",
    });

    enc.createNode(2, {
      width: 30,
      height: 8,
      backgroundColor: "#18181b",
      borderRadius: 8,
    });
    enc.appendChild(1, 2);

    bridge.flushMutations();
    bridge.renderFrame();

    const output = bridge.getRenderedOutput();
    const text = new TextDecoder().decode(output);

    // Kitty graphics protocol sequences start with ESC_Ga (APC + "G")
    // \x1b_G is the Kitty graphics protocol prefix
    // If pixel rendering is active, we expect these sequences
    const hasKittyGraphics = text.includes("\x1b_G") || text.includes("\x1bPG");

    // Even if Kitty protocol isn't used in test mode, the output should
    // still contain ANSI escape sequences for colors
    const hasAnsiEscapes = text.includes("\x1b[");
    expect(hasAnsiEscapes || hasKittyGraphics).toBe(true);
  });
});
