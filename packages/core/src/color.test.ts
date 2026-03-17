import { describe, expect, it } from "bun:test";
import { parseColor } from "./color.js";

describe("parseColor", () => {
  // -----------------------------------------------------------------------
  // Hex colors
  // -----------------------------------------------------------------------

  describe("hex", () => {
    it("parses #rrggbb", () => {
      expect(parseColor("#ff0000")).toEqual({ type: "rgb", r: 255, g: 0, b: 0 });
    });

    it("parses #rgb shorthand", () => {
      expect(parseColor("#f00")).toEqual({ type: "rgb", r: 255, g: 0, b: 0 });
    });

    it("parses #000000", () => {
      expect(parseColor("#000000")).toEqual({ type: "rgb", r: 0, g: 0, b: 0 });
    });

    it("parses mixed case", () => {
      expect(parseColor("#FfAa00")).toEqual({ type: "rgb", r: 255, g: 170, b: 0 });
    });

    it("returns undefined for invalid hex", () => {
      expect(parseColor("#xyz")).toBeUndefined();
    });

    it("returns undefined for wrong length", () => {
      expect(parseColor("#1234")).toBeUndefined();
    });
  });

  // -----------------------------------------------------------------------
  // rgb() function
  // -----------------------------------------------------------------------

  describe("rgb()", () => {
    it("parses rgb(r, g, b)", () => {
      expect(parseColor("rgb(255, 128, 0)")).toEqual({ type: "rgb", r: 255, g: 128, b: 0 });
    });

    it("parses without spaces", () => {
      expect(parseColor("rgb(0,0,0)")).toEqual({ type: "rgb", r: 0, g: 0, b: 0 });
    });

    it("returns undefined for out of range", () => {
      expect(parseColor("rgb(256, 0, 0)")).toBeUndefined();
    });

    it("returns undefined for negative", () => {
      expect(parseColor("rgb(-1, 0, 0)")).toBeUndefined();
    });

    it("returns undefined for wrong count", () => {
      expect(parseColor("rgb(1, 2)")).toBeUndefined();
    });
  });

  // -----------------------------------------------------------------------
  // Named colors
  // -----------------------------------------------------------------------

  describe("named colors", () => {
    it("parses 'red'", () => {
      expect(parseColor("red")).toEqual({ type: "rgb", r: 255, g: 0, b: 0 });
    });

    it("parses 'blue'", () => {
      expect(parseColor("blue")).toEqual({ type: "rgb", r: 0, g: 0, b: 255 });
    });

    it("is case insensitive", () => {
      expect(parseColor("RED")).toEqual({ type: "rgb", r: 255, g: 0, b: 0 });
    });

    it("trims whitespace", () => {
      expect(parseColor("  green  ")).toEqual({ type: "rgb", r: 0, g: 128, b: 0 });
    });

    it("returns undefined for unknown name", () => {
      expect(parseColor("unicorn")).toBeUndefined();
    });
  });

  // -----------------------------------------------------------------------
  // ANSI 256 palette
  // -----------------------------------------------------------------------

  describe("ansi()", () => {
    it("parses ansi(0)", () => {
      expect(parseColor("ansi(0)")).toEqual({ type: "palette", index: 0 });
    });

    it("parses ansi(255)", () => {
      expect(parseColor("ansi(255)")).toEqual({ type: "palette", index: 255 });
    });

    it("returns undefined for out of range", () => {
      expect(parseColor("ansi(256)")).toBeUndefined();
    });

    it("returns undefined for negative", () => {
      expect(parseColor("ansi(-1)")).toBeUndefined();
    });
  });

  // -----------------------------------------------------------------------
  // ANSI standard / bright
  // -----------------------------------------------------------------------

  describe("ansi-standard()", () => {
    it("parses ansi-standard(0)", () => {
      expect(parseColor("ansi-standard(0)")).toEqual({ type: "ansi", index: 0 });
    });

    it("parses ansi-standard(7)", () => {
      expect(parseColor("ansi-standard(7)")).toEqual({ type: "ansi", index: 7 });
    });

    it("returns undefined for out of range", () => {
      expect(parseColor("ansi-standard(8)")).toBeUndefined();
    });
  });

  describe("ansi-bright()", () => {
    it("parses ansi-bright(0)", () => {
      expect(parseColor("ansi-bright(0)")).toEqual({ type: "ansi-bright", index: 0 });
    });

    it("parses ansi-bright(7)", () => {
      expect(parseColor("ansi-bright(7)")).toEqual({ type: "ansi-bright", index: 7 });
    });

    it("returns undefined for out of range", () => {
      expect(parseColor("ansi-bright(8)")).toBeUndefined();
    });
  });
});
