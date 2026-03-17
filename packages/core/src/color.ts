/**
 * Color parsing utilities.
 *
 * Accepts CSS-like color values (hex, rgb(), named colors, ANSI 256 palette)
 * and converts them to the Color type used by the rendering engine.
 */

import type { Color } from "./types.js";

// ---------------------------------------------------------------------------
// Named CSS colors (W3C basic + extended subset commonly used in terminals)
// ---------------------------------------------------------------------------

const NAMED_COLORS: Record<string, [number, number, number]> = {
  black: [0, 0, 0],
  red: [255, 0, 0],
  green: [0, 128, 0],
  blue: [0, 0, 255],
  white: [255, 255, 255],
  yellow: [255, 255, 0],
  cyan: [0, 255, 255],
  magenta: [255, 0, 255],
  orange: [255, 165, 0],
  purple: [128, 0, 128],
  pink: [255, 192, 203],
  brown: [165, 42, 42],
  gray: [128, 128, 128],
  grey: [128, 128, 128],
  lime: [0, 255, 0],
  navy: [0, 0, 128],
  teal: [0, 128, 128],
  olive: [128, 128, 0],
  maroon: [128, 0, 0],
  aqua: [0, 255, 255],
  fuchsia: [255, 0, 255],
  silver: [192, 192, 192],
  transparent: [0, 0, 0],

  // ANSI named colors (mapped to standard palette)
  "ansi-black": [0, 0, 0],
  "ansi-red": [170, 0, 0],
  "ansi-green": [0, 170, 0],
  "ansi-yellow": [170, 85, 0],
  "ansi-blue": [0, 0, 170],
  "ansi-magenta": [170, 0, 170],
  "ansi-cyan": [0, 170, 170],
  "ansi-white": [170, 170, 170],

  // Bright ANSI
  "ansi-bright-black": [85, 85, 85],
  "ansi-bright-red": [255, 85, 85],
  "ansi-bright-green": [85, 255, 85],
  "ansi-bright-yellow": [255, 255, 85],
  "ansi-bright-blue": [85, 85, 255],
  "ansi-bright-magenta": [255, 85, 255],
  "ansi-bright-cyan": [85, 255, 255],
  "ansi-bright-white": [255, 255, 255],
};

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/**
 * Parse a color string into a Color value.
 *
 * Supported formats:
 * - Hex: `#rgb`, `#rrggbb`
 * - RGB function: `rgb(r, g, b)`
 * - Named CSS colors: `red`, `blue`, etc.
 * - ANSI 256 palette: `ansi(0)` through `ansi(255)`
 * - ANSI standard: `ansi-standard(0)` through `ansi-standard(7)`
 * - ANSI bright: `ansi-bright(0)` through `ansi-bright(7)`
 *
 * Returns `undefined` if the string cannot be parsed.
 */
export function parseColor(input: string): Color | undefined {
  const s = input.trim().toLowerCase();

  // Hex: #rgb or #rrggbb
  if (s.startsWith("#")) {
    return parseHex(s);
  }

  // rgb(r, g, b)
  if (s.startsWith("rgb(") && s.endsWith(")")) {
    return parseRgbFunc(s);
  }

  // ansi(n) — 256-color palette
  if (s.startsWith("ansi(") && s.endsWith(")")) {
    return parseAnsiFunc(s);
  }

  // ansi-standard(n)
  if (s.startsWith("ansi-standard(") && s.endsWith(")")) {
    return parseAnsiStandardFunc(s);
  }

  // ansi-bright(n)
  if (s.startsWith("ansi-bright(") && s.endsWith(")")) {
    return parseAnsiBrightFunc(s);
  }

  // Named color
  const named = NAMED_COLORS[s];
  if (named) {
    return { type: "rgb", r: named[0], g: named[1], b: named[2] };
  }

  return undefined;
}

function parseHex(s: string): Color | undefined {
  const hex = s.slice(1);
  if (hex.length === 3) {
    const r = Number.parseInt(hex[0] + hex[0], 16);
    const g = Number.parseInt(hex[1] + hex[1], 16);
    const b = Number.parseInt(hex[2] + hex[2], 16);
    if (Number.isNaN(r) || Number.isNaN(g) || Number.isNaN(b)) return undefined;
    return { type: "rgb", r, g, b };
  }
  if (hex.length === 6) {
    const r = Number.parseInt(hex.slice(0, 2), 16);
    const g = Number.parseInt(hex.slice(2, 4), 16);
    const b = Number.parseInt(hex.slice(4, 6), 16);
    if (Number.isNaN(r) || Number.isNaN(g) || Number.isNaN(b)) return undefined;
    return { type: "rgb", r, g, b };
  }
  return undefined;
}

function parseRgbFunc(s: string): Color | undefined {
  const inner = s.slice(4, -1);
  const parts = inner.split(",").map((p) => p.trim());
  if (parts.length !== 3) return undefined;
  const r = Number.parseInt(parts[0], 10);
  const g = Number.parseInt(parts[1], 10);
  const b = Number.parseInt(parts[2], 10);
  if (Number.isNaN(r) || Number.isNaN(g) || Number.isNaN(b)) return undefined;
  if (r < 0 || r > 255 || g < 0 || g > 255 || b < 0 || b > 255) return undefined;
  return { type: "rgb", r, g, b };
}

function parseAnsiFunc(s: string): Color | undefined {
  const inner = s.slice(5, -1).trim();
  const n = Number.parseInt(inner, 10);
  if (Number.isNaN(n) || n < 0 || n > 255) return undefined;
  return { type: "palette", index: n };
}

function parseAnsiStandardFunc(s: string): Color | undefined {
  const inner = s.slice(14, -1).trim();
  const n = Number.parseInt(inner, 10);
  if (Number.isNaN(n) || n < 0 || n > 7) return undefined;
  return { type: "ansi", index: n };
}

function parseAnsiBrightFunc(s: string): Color | undefined {
  const inner = s.slice(12, -1).trim();
  const n = Number.parseInt(inner, 10);
  if (Number.isNaN(n) || n < 0 || n > 7) return undefined;
  return { type: "ansi-bright", index: n };
}
