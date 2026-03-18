import { describe, expect, it, beforeEach } from "bun:test";
import { resetNodeIdCounter } from "./renderable.js";
import {
  TextRenderable,
  resolveSpans,
  wrapText,
  truncateLine,
  alignLine,
  measureText,
  type StyledChar,
  type TextSpan,
} from "./text.js";
import type { TextStyle } from "./types.js";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Extract plain text from an array of StyledChar. */
const toText = (chars: StyledChar[]): string => chars.map((c) => c.ch).join("");

/** Create unstyled StyledChars from a string. */
const plain = (str: string): StyledChar[] =>
  Array.from(str, (ch) => ({ ch, style: {} }));

// ---------------------------------------------------------------------------
// resolveSpans
// ---------------------------------------------------------------------------

describe("resolveSpans", () => {
  it("applies base style to all characters", () => {
    const base: TextStyle = { bold: true };
    const result = resolveSpans("abc", base, []);
    expect(result).toHaveLength(3);
    expect(result[0].style.bold).toBe(true);
    expect(result[1].style.bold).toBe(true);
    expect(result[2].style.bold).toBe(true);
  });

  it("applies a span overlay", () => {
    const base: TextStyle = {};
    const spans: TextSpan[] = [{ start: 1, end: 3, style: { italic: true } }];
    const result = resolveSpans("abcd", base, spans);
    expect(result[0].style.italic).toBeUndefined();
    expect(result[1].style.italic).toBe(true);
    expect(result[2].style.italic).toBe(true);
    expect(result[3].style.italic).toBeUndefined();
  });

  it("later spans override earlier spans", () => {
    const base: TextStyle = {};
    const red: TextStyle = { fg: { type: "rgb", r: 255, g: 0, b: 0 } };
    const blue: TextStyle = { fg: { type: "rgb", r: 0, g: 0, b: 255 } };
    const spans: TextSpan[] = [
      { start: 0, end: 4, style: red },
      { start: 2, end: 4, style: blue },
    ];
    const result = resolveSpans("abcd", base, spans);
    expect(result[0].style.fg).toEqual(red.fg);
    expect(result[1].style.fg).toEqual(red.fg);
    expect(result[2].style.fg).toEqual(blue.fg);
    expect(result[3].style.fg).toEqual(blue.fg);
  });

  it("clamps span indices to text bounds", () => {
    const result = resolveSpans("ab", {}, [
      { start: -5, end: 100, style: { bold: true } },
    ]);
    expect(result).toHaveLength(2);
    expect(result[0].style.bold).toBe(true);
    expect(result[1].style.bold).toBe(true);
  });

  it("returns empty array for empty text", () => {
    const result = resolveSpans("", {}, []);
    expect(result).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// wrapText
// ---------------------------------------------------------------------------

describe("wrapText", () => {
  it("no-wrap keeps text on one line per hard newline", () => {
    const chars = plain("hello world");
    const lines = wrapText(chars, "no-wrap", 5);
    expect(lines).toHaveLength(1);
    expect(toText(lines[0])).toBe("hello world");
  });

  it("no-wrap splits on hard newlines", () => {
    const chars = plain("line1\nline2");
    const lines = wrapText(chars, "no-wrap", 5);
    expect(lines).toHaveLength(2);
    expect(toText(lines[0])).toBe("line1");
    expect(toText(lines[1])).toBe("line2");
  });

  it("break-word breaks at exact character boundaries", () => {
    const chars = plain("abcdefgh");
    const lines = wrapText(chars, "break-word", 3);
    expect(lines).toHaveLength(3);
    expect(toText(lines[0])).toBe("abc");
    expect(toText(lines[1])).toBe("def");
    expect(toText(lines[2])).toBe("gh");
  });

  it("word-wrap breaks at word boundaries", () => {
    const chars = plain("hello world foo");
    const lines = wrapText(chars, "word-wrap", 11);
    expect(lines).toHaveLength(2);
    expect(toText(lines[0])).toBe("hello world");
    expect(toText(lines[1])).toBe("foo");
  });

  it("word-wrap breaks long words", () => {
    const chars = plain("abcdefghij");
    const lines = wrapText(chars, "word-wrap", 4);
    expect(lines.length).toBeGreaterThanOrEqual(2);
    // First chunks should be 4 chars
    expect(toText(lines[0])).toBe("abcd");
    expect(toText(lines[1])).toBe("efgh");
    expect(toText(lines[2])).toBe("ij");
  });

  it("handles empty text", () => {
    const lines = wrapText([], "word-wrap", 10);
    expect(lines).toHaveLength(1);
    expect(lines[0]).toHaveLength(0);
  });

  it("handles text with no maxWidth", () => {
    const chars = plain("hello world");
    const lines = wrapText(chars, "word-wrap", undefined);
    expect(lines).toHaveLength(1);
    expect(toText(lines[0])).toBe("hello world");
  });
});

// ---------------------------------------------------------------------------
// truncateLine
// ---------------------------------------------------------------------------

describe("truncateLine", () => {
  it("returns line unchanged if within width", () => {
    const line = plain("abc");
    const result = truncateLine(line, 5, "clip");
    expect(toText(result)).toBe("abc");
  });

  it("clips to maxWidth", () => {
    const line = plain("abcdef");
    const result = truncateLine(line, 3, "clip");
    expect(toText(result)).toBe("abc");
  });

  it("truncates with ellipsis", () => {
    const line = plain("abcdef");
    const result = truncateLine(line, 3, "ellipsis");
    expect(result).toHaveLength(3);
    expect(result[2].ch).toBe("\u2026");
    expect(toText(result)).toBe("ab\u2026");
  });

  it("handles maxWidth of 1 with ellipsis", () => {
    const line = plain("abc");
    const result = truncateLine(line, 1, "ellipsis");
    expect(result).toHaveLength(1);
    expect(result[0].ch).toBe("\u2026");
  });
});

// ---------------------------------------------------------------------------
// alignLine
// ---------------------------------------------------------------------------

describe("alignLine", () => {
  const fillStyle: TextStyle = {};

  it("left alignment returns line as-is", () => {
    const line = plain("hi");
    const result = alignLine({ line, width: 5, align: "left", fillStyle });
    expect(toText(result)).toBe("hi");
  });

  it("right alignment pads on the left", () => {
    const line = plain("hi");
    const result = alignLine({ line, width: 5, align: "right", fillStyle });
    expect(toText(result)).toBe("   hi");
    expect(result).toHaveLength(5);
  });

  it("center alignment pads on both sides", () => {
    const line = plain("hi");
    const result = alignLine({ line, width: 6, align: "center", fillStyle });
    expect(result).toHaveLength(6);
    expect(toText(result)).toBe("  hi  ");
  });

  it("center alignment with odd gap favors left", () => {
    const line = plain("hi");
    const result = alignLine({ line, width: 5, align: "center", fillStyle });
    expect(result).toHaveLength(5);
    expect(toText(result)).toBe(" hi  ");
  });

  it("does not pad if line is already wider", () => {
    const line = plain("hello");
    const result = alignLine({ line, width: 3, align: "center", fillStyle });
    expect(toText(result)).toBe("hello");
  });
});

// ---------------------------------------------------------------------------
// measureText
// ---------------------------------------------------------------------------

describe("measureText", () => {
  it("measures simple text", () => {
    const m = measureText("hello", "no-wrap");
    expect(m.width).toBe(5);
    expect(m.height).toBe(1);
  });

  it("measures multi-line text", () => {
    const m = measureText("hello\nworld!", "no-wrap");
    expect(m.width).toBe(6); // "world!" is 6 chars
    expect(m.height).toBe(2);
  });

  it("measures with wrapping constraint", () => {
    const m = measureText("hello world", "word-wrap", 6);
    expect(m.height).toBe(2);
    expect(m.width).toBeLessThanOrEqual(6);
  });

  it("measures empty text", () => {
    const m = measureText("", "no-wrap");
    expect(m.width).toBe(0);
    expect(m.height).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// TextRenderable
// ---------------------------------------------------------------------------

describe("TextRenderable", () => {
  beforeEach(() => {
    resetNodeIdCounter();
  });

  it("constructs with defaults", () => {
    const t = new TextRenderable();
    expect(t.wrap).toBe("word-wrap");
    expect(t.overflow).toBe("clip");
    expect(t.align).toBe("left");
    expect(t.spans).toHaveLength(0);
    expect(t.text).toBeUndefined();
  });

  it("constructs with options", () => {
    const t = new TextRenderable({
      text: "hello",
      wrap: "no-wrap",
      overflow: "ellipsis",
      align: "center",
      spans: [{ start: 0, end: 2, style: { bold: true } }],
    });
    expect(t.text).toBe("hello");
    expect(t.wrap).toBe("no-wrap");
    expect(t.overflow).toBe("ellipsis");
    expect(t.align).toBe("center");
    expect(t.spans).toHaveLength(1);
  });

  it("assigns a node ID", () => {
    const t = new TextRenderable();
    expect(t.nodeId).toBe(1);
  });

  it("setWrap marks dirty", () => {
    const t = new TextRenderable();
    t.clearDirty();
    t.setWrap("break-word");
    expect(t.dirty).toBe(true);
    expect(t.wrap).toBe("break-word");
  });

  it("setWrap does not mark dirty if same value", () => {
    const t = new TextRenderable({ wrap: "no-wrap" });
    t.clearDirty();
    t.setWrap("no-wrap");
    expect(t.dirty).toBe(false);
  });

  it("setOverflow marks dirty", () => {
    const t = new TextRenderable();
    t.clearDirty();
    t.setOverflow("ellipsis");
    expect(t.dirty).toBe(true);
  });

  it("setAlign marks dirty", () => {
    const t = new TextRenderable();
    t.clearDirty();
    t.setAlign("center");
    expect(t.dirty).toBe(true);
  });

  it("setSpans marks dirty", () => {
    const t = new TextRenderable();
    t.clearDirty();
    t.setSpans([{ start: 0, end: 1, style: { bold: true } }]);
    expect(t.dirty).toBe(true);
  });

  it("addSpan marks dirty", () => {
    const t = new TextRenderable();
    t.clearDirty();
    t.addSpan({ start: 0, end: 1, style: { italic: true } });
    expect(t.dirty).toBe(true);
    expect(t.spans).toHaveLength(1);
  });

  it("clearSpans marks dirty only if non-empty", () => {
    const t = new TextRenderable();
    t.clearDirty();
    t.clearSpans();
    expect(t.dirty).toBe(false);

    t.addSpan({ start: 0, end: 1, style: {} });
    t.clearDirty();
    t.clearSpans();
    expect(t.dirty).toBe(true);
    expect(t.spans).toHaveLength(0);
  });

  // -----------------------------------------------------------------------
  // Measurement
  // -----------------------------------------------------------------------

  it("measure returns intrinsic size", () => {
    const t = new TextRenderable({ text: "hello" });
    const m = t.measure();
    expect(m.width).toBe(5);
    expect(m.height).toBe(1);
  });

  it("measure with maxWidth wraps text", () => {
    const t = new TextRenderable({ text: "hello world" });
    const m = t.measure(6);
    expect(m.height).toBe(2);
  });

  it("measure with empty text", () => {
    const t = new TextRenderable();
    const m = t.measure();
    expect(m.width).toBe(0);
    expect(m.height).toBe(0);
  });

  // -----------------------------------------------------------------------
  // Rendering
  // -----------------------------------------------------------------------

  it("render produces styled lines", () => {
    const t = new TextRenderable({ text: "hello" });
    const lines = t.render(10);
    expect(lines).toHaveLength(1);
    expect(toText(lines[0])).toBe("hello");
  });

  it("render wraps text", () => {
    const t = new TextRenderable({ text: "hello world", wrap: "word-wrap" });
    const lines = t.render(6);
    expect(lines).toHaveLength(2);
    expect(toText(lines[0]).trimEnd()).toBe("hello");
  });

  it("render truncates with ellipsis", () => {
    const t = new TextRenderable({
      text: "hello world",
      wrap: "no-wrap",
      overflow: "ellipsis",
    });
    const lines = t.render(5);
    expect(lines).toHaveLength(1);
    expect(lines[0]).toHaveLength(5);
    expect(lines[0][4].ch).toBe("\u2026");
  });

  it("render aligns center", () => {
    const t = new TextRenderable({
      text: "hi",
      align: "center",
    });
    const lines = t.render(6);
    expect(lines).toHaveLength(1);
    expect(toText(lines[0])).toBe("  hi  ");
  });

  it("render aligns right", () => {
    const t = new TextRenderable({
      text: "hi",
      align: "right",
    });
    const lines = t.render(5);
    expect(lines).toHaveLength(1);
    expect(toText(lines[0])).toBe("   hi");
  });

  it("render applies inline spans", () => {
    const t = new TextRenderable({
      text: "abcd",
      spans: [{ start: 1, end: 3, style: { bold: true } }],
    });
    const lines = t.render();
    expect(lines).toHaveLength(1);
    expect(lines[0][0].style.bold).toBeUndefined();
    expect(lines[0][1].style.bold).toBe(true);
    expect(lines[0][2].style.bold).toBe(true);
    expect(lines[0][3].style.bold).toBeUndefined();
  });

  it("render returns empty for empty text", () => {
    const t = new TextRenderable();
    const lines = t.render(10);
    expect(lines).toHaveLength(0);
  });

  it("render uses layout width when no maxWidth given", () => {
    const t = new TextRenderable({ text: "hello world" });
    t.updateLayout({ x: 0, y: 0, width: 6, height: 2 });
    const lines = t.render();
    expect(lines).toHaveLength(2);
  });

  // -----------------------------------------------------------------------
  // Style integration
  // -----------------------------------------------------------------------

  it("inherits text style from setStyle", () => {
    const t = new TextRenderable({ text: "hi" });
    t.setStyle({ color: "red", fontWeight: "bold" });
    const lines = t.render();
    expect(lines[0][0].style.fg).toEqual({ type: "rgb", r: 255, g: 0, b: 0 });
    expect(lines[0][0].style.bold).toBe(true);
  });
});
