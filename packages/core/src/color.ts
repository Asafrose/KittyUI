/**
 * Color parsing utilities.
 *
 * Accepts CSS-like color values (hex, rgb(), named colors, ANSI 256 palette)
 * and converts them to the Color type used by the rendering engine.
 */

import type { Color } from "./types.js";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const HEX_SHORT_LEN = 3;
const HEX_LONG_LEN = 6;
const HEX_RADIX = 16;
const DECIMAL_RADIX = 10;
const RGB_COMPONENT_COUNT = 3;
const RGB_PREFIX_LEN = 4;
const ANSI_PREFIX_LEN = 5;
const ANSI_STANDARD_PREFIX_LEN = 14;
const ANSI_BRIGHT_PREFIX_LEN = 12;
const MAX_COLOR_VALUE = 255;
const MAX_ANSI_STANDARD = 7;

// ---------------------------------------------------------------------------
// Named CSS colors (W3C basic + extended subset commonly used in terminals)
// ---------------------------------------------------------------------------

/* eslint-disable no-magic-numbers */
const NAMED_COLORS: Record<string, [number, number, number]> = {
  "ansi-black": [0, 0, 0],
  "ansi-blue": [0, 0, 170],
  "ansi-bright-black": [85, 85, 85],
  "ansi-bright-blue": [85, 85, 255],
  "ansi-bright-cyan": [85, 255, 255],
  "ansi-bright-green": [85, 255, 85],
  "ansi-bright-magenta": [255, 85, 255],
  "ansi-bright-red": [255, 85, 85],
  "ansi-bright-white": [255, 255, 255],
  "ansi-bright-yellow": [255, 255, 85],
  "ansi-cyan": [0, 170, 170],
  "ansi-green": [0, 170, 0],
  "ansi-magenta": [170, 0, 170],
  "ansi-red": [170, 0, 0],
  "ansi-white": [170, 170, 170],
  "ansi-yellow": [170, 85, 0],
  aqua: [0, 255, 255],
  black: [0, 0, 0],
  blue: [0, 0, 255],
  brown: [165, 42, 42],
  cyan: [0, 255, 255],
  fuchsia: [255, 0, 255],
  gray: [128, 128, 128],
  green: [0, 128, 0],
  grey: [128, 128, 128],
  lime: [0, 255, 0],
  magenta: [255, 0, 255],
  maroon: [128, 0, 0],
  navy: [0, 0, 128],
  olive: [128, 128, 0],
  orange: [255, 165, 0],
  pink: [255, 192, 203],
  purple: [128, 0, 128],
  red: [255, 0, 0],
  silver: [192, 192, 192],
  teal: [0, 128, 128],
  transparent: [0, 0, 0],
  white: [255, 255, 255],
  yellow: [255, 255, 0],
};
/* eslint-enable no-magic-numbers */

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
export const parseColor = (input: string): Color | undefined => {
  const str = input.trim().toLowerCase();

  if (str.startsWith("#")) {
    return parseHex(str);
  }

  if (str.startsWith("rgb(") && str.endsWith(")")) {
    return parseRgbFunc(str);
  }

  if (str.startsWith("ansi(") && str.endsWith(")")) {
    return parseAnsiFunc(str);
  }

  if (str.startsWith("ansi-standard(") && str.endsWith(")")) {
    return parseAnsiStandardFunc(str);
  }

  if (str.startsWith("ansi-bright(") && str.endsWith(")")) {
    return parseAnsiBrightFunc(str);
  }

  const named = NAMED_COLORS[str];
  if (named) {
    return { b: named[2], g: named[1], r: named[0], type: "rgb" };
  }

  return undefined;
};

const parseHex = (str: string): Color | undefined => {
  const hex = str.slice(1);
  if (hex.length === HEX_SHORT_LEN) {
    const rr = Number.parseInt(hex[0] + hex[0], HEX_RADIX);
    const gg = Number.parseInt(hex[1] + hex[1], HEX_RADIX);
    const bb = Number.parseInt(hex[2] + hex[2], HEX_RADIX);
    if (Number.isNaN(rr) || Number.isNaN(gg) || Number.isNaN(bb)) {
      return undefined;
    }
    return { b: bb, g: gg, r: rr, type: "rgb" };
  }
  if (hex.length === HEX_LONG_LEN) {
    const rr = Number.parseInt(hex.slice(0, 2), HEX_RADIX);
    const gg = Number.parseInt(hex.slice(2, 4), HEX_RADIX);
    const bb = Number.parseInt(hex.slice(4, 6), HEX_RADIX);
    if (Number.isNaN(rr) || Number.isNaN(gg) || Number.isNaN(bb)) {
      return undefined;
    }
    return { b: bb, g: gg, r: rr, type: "rgb" };
  }
  return undefined;
};

const parseRgbFunc = (str: string): Color | undefined => {
  const inner = str.slice(RGB_PREFIX_LEN, -1);
  const parts = inner.split(",").map((pp) => pp.trim());
  if (parts.length !== RGB_COMPONENT_COUNT) {
    return undefined;
  }
  const rr = Number.parseInt(parts[0], DECIMAL_RADIX);
  const gg = Number.parseInt(parts[1], DECIMAL_RADIX);
  const bb = Number.parseInt(parts[2], DECIMAL_RADIX);
  if (Number.isNaN(rr) || Number.isNaN(gg) || Number.isNaN(bb)) {
    return undefined;
  }
  if (rr < 0 || rr > MAX_COLOR_VALUE || gg < 0 || gg > MAX_COLOR_VALUE || bb < 0 || bb > MAX_COLOR_VALUE) {
    return undefined;
  }
  return { b: bb, g: gg, r: rr, type: "rgb" };
};

const parseAnsiFunc = (str: string): Color | undefined => {
  const inner = str.slice(ANSI_PREFIX_LEN, -1).trim();
  const nn = Number.parseInt(inner, DECIMAL_RADIX);
  if (Number.isNaN(nn) || nn < 0 || nn > MAX_COLOR_VALUE) {
    return undefined;
  }
  return { index: nn, type: "palette" };
};

const parseAnsiStandardFunc = (str: string): Color | undefined => {
  const inner = str.slice(ANSI_STANDARD_PREFIX_LEN, -1).trim();
  const nn = Number.parseInt(inner, DECIMAL_RADIX);
  if (Number.isNaN(nn) || nn < 0 || nn > MAX_ANSI_STANDARD) {
    return undefined;
  }
  return { index: nn, type: "ansi" };
};

const parseAnsiBrightFunc = (str: string): Color | undefined => {
  const inner = str.slice(ANSI_BRIGHT_PREFIX_LEN, -1).trim();
  const nn = Number.parseInt(inner, DECIMAL_RADIX);
  if (Number.isNaN(nn) || nn < 0 || nn > MAX_ANSI_STANDARD) {
    return undefined;
  }
  return { index: nn, type: "ansi-bright" };
};
