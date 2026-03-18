import { describe, expect, it } from "bun:test";
import { normalizeStyle, parseDim } from "./style.js";
import type { CSSStyle } from "./style.js";

describe("parseDim", () => {
  it("parses a number as cells", () => {
    expect(parseDim(10)).toEqual({ type: "cells", value: 10 });
  });

  it("parses 'auto'", () => {
    expect(parseDim("auto")).toEqual({ type: "auto" });
  });

  it("parses percentage string", () => {
    expect(parseDim("50%")).toEqual({ type: "percent", value: 50 });
  });

  it("parses numeric string as cells", () => {
    expect(parseDim("20")).toEqual({ type: "cells", value: 20 });
  });

  it("fallback to auto for invalid string", () => {
    expect(parseDim("invalid")).toEqual({ type: "auto" });
  });
});

describe("normalizeStyle", () => {
  // -----------------------------------------------------------------------
  // Display mode
  // -----------------------------------------------------------------------

  it("defaults to flex display", () => {
    const { node } = normalizeStyle({});
    expect(node.display?.type).toBe("flex");
  });

  it("sets grid display", () => {
    const { node } = normalizeStyle({ display: "grid" });
    expect(node.display?.type).toBe("grid");
  });

  // -----------------------------------------------------------------------
  // Flex properties
  // -----------------------------------------------------------------------

  it("sets flex direction", () => {
    const { node } = normalizeStyle({ flexDirection: "column" });
    expect(node.display).toEqual({
      type: "flex",
      flex: { direction: "column" },
    });
  });

  it("sets flex grow and shrink", () => {
    const { node } = normalizeStyle({ flexGrow: 1, flexShrink: 0 });
    const display = node.display as { type: "flex"; flex: Record<string, unknown> };
    expect(display.flex.grow).toBe(1);
    expect(display.flex.shrink).toBe(0);
  });

  // -----------------------------------------------------------------------
  // Sizing
  // -----------------------------------------------------------------------

  it("sets width and height", () => {
    const { node } = normalizeStyle({ width: 80, height: 24 });
    expect(node.width).toEqual({ type: "cells", value: 80 });
    expect(node.height).toEqual({ type: "cells", value: 24 });
  });

  it("sets percentage width", () => {
    const { node } = normalizeStyle({ width: "50%" });
    expect(node.width).toEqual({ type: "percent", value: 50 });
  });

  it("sets auto width", () => {
    const { node } = normalizeStyle({ width: "auto" });
    expect(node.width).toEqual({ type: "auto" });
  });

  // -----------------------------------------------------------------------
  // Padding shorthand
  // -----------------------------------------------------------------------

  it("expands single padding value to all sides", () => {
    const { node } = normalizeStyle({ padding: 2 });
    const d = { type: "cells" as const, value: 2 };
    expect(node.padding).toEqual([d, d, d, d]);
  });

  it("expands two-value padding to [v, h, v, h]", () => {
    const { node } = normalizeStyle({ padding: [1, 2] });
    expect(node.padding).toEqual([
      { type: "cells" as const, value: 1 },
      { type: "cells" as const, value: 2 },
      { type: "cells" as const, value: 1 },
      { type: "cells" as const, value: 2 },
    ]);
  });

  it("passes four-value padding through", () => {
    const { node } = normalizeStyle({ padding: [1, 2, 3, 4] });
    expect(node.padding).toEqual([
      { type: "cells" as const, value: 1 },
      { type: "cells" as const, value: 2 },
      { type: "cells" as const, value: 3 },
      { type: "cells" as const, value: 4 },
    ]);
  });

  it("individual padding overrides shorthand", () => {
    const { node } = normalizeStyle({ padding: 1, paddingLeft: 5 });
    expect(node.padding![3]).toEqual({ type: "cells", value: 5 });
    expect(node.padding![0]).toEqual({ type: "cells", value: 1 });
  });

  // -----------------------------------------------------------------------
  // Margin shorthand
  // -----------------------------------------------------------------------

  it("expands single margin value", () => {
    const { node } = normalizeStyle({ margin: 3 });
    const d = { type: "cells" as const, value: 3 };
    expect(node.margin).toEqual([d, d, d, d]);
  });

  // -----------------------------------------------------------------------
  // Gap
  // -----------------------------------------------------------------------

  it("expands single gap value", () => {
    const { node } = normalizeStyle({ gap: 5 });
    const d = { type: "cells" as const, value: 5 };
    expect(node.gap).toEqual([d, d]);
  });

  it("expands two-value gap", () => {
    const { node } = normalizeStyle({ gap: [5, 10] });
    expect(node.gap).toEqual([
      { type: "cells" as const, value: 5 },
      { type: "cells" as const, value: 10 },
    ]);
  });

  // -----------------------------------------------------------------------
  // Grid properties
  // -----------------------------------------------------------------------

  it("normalizes grid template columns", () => {
    const { node } = normalizeStyle({
      display: "grid",
      gridTemplateColumns: [20, "1fr", "auto"],
    });
    const display = node.display as { type: "grid"; grid: Record<string, unknown> };
    expect(display.grid.columns).toEqual([
      { type: "cells", value: 20 },
      { type: "fr", value: 1 },
      { type: "auto" },
    ]);
  });

  // -----------------------------------------------------------------------
  // Text style
  // -----------------------------------------------------------------------

  it("resolves string color", () => {
    const { text } = normalizeStyle({ color: "#ff0000" });
    expect(text.fg).toEqual({ type: "rgb", r: 255, g: 0, b: 0 });
  });

  it("resolves named color", () => {
    const { text } = normalizeStyle({ color: "blue" });
    expect(text.fg).toEqual({ type: "rgb", r: 0, g: 0, b: 255 });
  });

  it("passes Color object through", () => {
    const color = { type: "ansi" as const, index: 1 };
    const { text } = normalizeStyle({ color });
    expect(text.fg).toEqual(color);
  });

  it("sets bold from fontWeight", () => {
    const { text } = normalizeStyle({ fontWeight: "bold" });
    expect(text.bold).toBe(true);
  });

  it("sets italic from fontStyle", () => {
    const { text } = normalizeStyle({ fontStyle: "italic" });
    expect(text.italic).toBe(true);
  });

  it("sets underline from textDecoration", () => {
    const { text } = normalizeStyle({ textDecoration: "underline" });
    expect(text.underline).toBe(true);
  });

  it("sets strikethrough from textDecoration", () => {
    const { text } = normalizeStyle({ textDecoration: "strikethrough" });
    expect(text.strikethrough).toBe(true);
  });

  it("sets overline from textDecoration", () => {
    const { text } = normalizeStyle({ textDecoration: "overline" });
    expect(text.overline).toBe(true);
  });

  it("sets underline style", () => {
    const { text } = normalizeStyle({ underlineStyle: "curly" });
    expect(text.underlineStyle).toBe("curly");
  });

  // -----------------------------------------------------------------------
  // Combination
  // -----------------------------------------------------------------------

  it("handles full CSS-like style", () => {
    const css: CSSStyle = {
      display: "flex",
      flexDirection: "column",
      justifyContent: "center",
      width: 80,
      height: 24,
      padding: [1, 2],
      color: "red",
      fontWeight: "bold",
    };
    const { node, text } = normalizeStyle(css);
    expect(node.width).toEqual({ type: "cells", value: 80 });
    expect(node.display?.type).toBe("flex");
    expect(text.bold).toBe(true);
    expect(text.fg).toEqual({ type: "rgb", r: 255, g: 0, b: 0 });
  });
});
