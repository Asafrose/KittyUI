/**
 * Style normalization — accepts CSS-like shorthand properties and converts
 * them to the canonical NodeStyle type used by the Taffy layout engine.
 */

import type { Color, Dim, FlexStyle, GridStyle, NodeStyle, TextStyle, TrackDef } from "./types.js";
import { parseColor } from "./color.js";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const SPACING_PAIR_LEN = 2;
const ZERO_DIM: Dim = { type: "cells", value: 0 };

// ---------------------------------------------------------------------------
// Border character input (mirrors BorderChars from box.ts to avoid circular deps)
// ---------------------------------------------------------------------------

/** Characters used to render a border (compatible with BoxRenderable's BorderChars). */
export interface BorderCharsInput {
  bottomLeft: string;
  bottomRight: string;
  horizontal: string;
  topLeft: string;
  topRight: string;
  vertical: string;
}

// ---------------------------------------------------------------------------
// CSS-like input style (what users write)
// ---------------------------------------------------------------------------

/**
 * A user-facing style object that accepts CSS-like shorthand properties.
 *
 * Shorthand expansions:
 * - `padding`: number | [vertical, horizontal] | [top, right, bottom, left]
 * - `margin`: same as padding
 * - `gap`: number | [column, row]
 * - `border`: not a Taffy concept, reserved for future use
 *
 * Dimension values can be:
 * - A number (interpreted as cells)
 * - A string like "50%" (percentage)
 * - "auto"
 */
export interface CSSStyle {
  alignItems?: "start" | "end" | "center" | "baseline" | "stretch" | undefined;
  /** Background: a color string, Color value, or CSS `linear-gradient(...)` expression. */
  background?: string | Color | undefined;
  backgroundColor?: string | Color | undefined;
  border?: "single" | "double" | "rounded" | "round" | "bold" | BorderCharsInput | false | undefined;
  borderColor?: string | Color | undefined;
  borderRadius?: number | undefined;
  color?: string | Color | undefined;
  columnGap?: DimInput | undefined;
  display?: "flex" | "grid" | undefined;
  flexBasis?: DimInput | undefined;
  flexDirection?: "row" | "column" | "row-reverse" | "column-reverse" | undefined;
  flexGrow?: number | undefined;
  flexShrink?: number | undefined;
  flexWrap?: "no-wrap" | "wrap" | "wrap-reverse" | undefined;
  fontStyle?: "normal" | "italic" | undefined;
  fontWeight?: "normal" | "bold" | undefined;
  gap?: DimInput | [DimInput, DimInput] | undefined;
  gridTemplateColumns?: GridTrackInput[] | undefined;
  gridTemplateRows?: GridTrackInput[] | undefined;
  height?: DimInput | undefined;
  justifyContent?:
    | "start"
    | "end"
    | "center"
    | "space-between"
    | "space-around"
    | "space-evenly"
    | undefined;
  margin?: DimInput | [DimInput, DimInput] | [DimInput, DimInput, DimInput, DimInput] | undefined;
  marginBottom?: DimInput | undefined;
  marginLeft?: DimInput | undefined;
  marginRight?: DimInput | undefined;
  marginTop?: DimInput | undefined;
  maxHeight?: DimInput | undefined;
  maxWidth?: DimInput | undefined;
  minHeight?: DimInput | undefined;
  minWidth?: DimInput | undefined;
  padding?: DimInput | [DimInput, DimInput] | [DimInput, DimInput, DimInput, DimInput] | undefined;
  paddingBottom?: DimInput | undefined;
  paddingLeft?: DimInput | undefined;
  paddingRight?: DimInput | undefined;
  paddingTop?: DimInput | undefined;
  rowGap?: DimInput | undefined;
  dim?: boolean | undefined;
  textDecoration?:
    | "none"
    | "underline"
    | "strikethrough"
    | "line-through"
    | "overline"
    | undefined;
  textOverflow?: "clip" | "ellipsis" | undefined;
  underlineColor?: string | Color | undefined;
  underlineStyle?:
    | "none"
    | "single"
    | "double"
    | "curly"
    | "dotted"
    | "dashed"
    | undefined;
  width?: DimInput | undefined;
}

/** Input for a dimension value. */
export type DimInput = number | string | "auto";

/** Input for a grid track definition. */
export type GridTrackInput = number | string | "auto";

// ---------------------------------------------------------------------------
// Normalization
// ---------------------------------------------------------------------------

/** Convert a DimInput to a Dim. */
export const parseDim = (input: DimInput): Dim => {
  if (input === "auto") {
    return { type: "auto" };
  }
  if (typeof input === "number") {
    return { type: "cells", value: input };
  }
  // String: check for percentage
  const trimmed = input.trim();
  if (trimmed === "auto") {
    return { type: "auto" };
  }
  if (trimmed.endsWith("%")) {
    const value = Number.parseFloat(trimmed);
    if (!Number.isNaN(value)) {
      return { type: "percent", value };
    }
  }
  // Try as number (cells)
  const num = Number.parseFloat(trimmed);
  if (!Number.isNaN(num)) {
    return { type: "cells", value: num };
  }
  return { type: "auto" };
};

/** Expand a shorthand spacing value into [top, right, bottom, left]. */
const expandSpacing = (
  input: DimInput | [DimInput, DimInput] | [DimInput, DimInput, DimInput, DimInput],
): [Dim, Dim, Dim, Dim] => {
  if (Array.isArray(input)) {
    if (input.length === SPACING_PAIR_LEN) {
      const vertical = parseDim(input[0]);
      const horizontal = parseDim(input[1]);
      return [vertical, horizontal, vertical, horizontal];
    }
    return [parseDim(input[0]), parseDim(input[1]), parseDim(input[2]), parseDim(input[3])];
  }
  const dim = parseDim(input);
  return [dim, dim, dim, dim];
};

/** Parse a grid track input into a TrackDef. */
const parseTrack = (input: GridTrackInput): TrackDef => {
  if (input === "auto") {
    return { type: "auto" };
  }
  if (typeof input === "number") {
    return { type: "cells", value: input };
  }
  const trimmed = input.trim();
  if (trimmed === "auto") {
    return { type: "auto" };
  }
  if (trimmed.endsWith("fr")) {
    const val = Number.parseFloat(trimmed);
    if (!Number.isNaN(val)) {
      return { type: "fr", value: val };
    }
  }
  if (trimmed.endsWith("%")) {
    const val = Number.parseFloat(trimmed);
    if (!Number.isNaN(val)) {
      return { type: "percent", value: val };
    }
  }
  const num = Number.parseFloat(trimmed);
  if (!Number.isNaN(num)) {
    return { type: "cells", value: num };
  }
  return { type: "auto" };
};

/** Resolve a color input (string or Color object) to a Color. */
const resolveColor = (input: string | Color | undefined): Color | undefined => {
  if (input === undefined) {
    return undefined;
  }
  if (typeof input === "string") {
    return parseColor(input);
  }
  return input;
};

/** Create a default zero-padding tuple. */
const defaultSpacing = (): [Dim, Dim, Dim, Dim] => [ZERO_DIM, ZERO_DIM, ZERO_DIM, ZERO_DIM];

/**
 * Normalize a CSS-like style object into the canonical NodeStyle + TextStyle.
 */
export const normalizeStyle = (css: CSSStyle): { node: NodeStyle; text: TextStyle } => {
  const node: NodeStyle = {};
  const text: TextStyle = {};

  // Display mode
  if (css.display === "grid") {
    const grid: GridStyle = {};
    if (css.gridTemplateColumns) {
      grid.columns = css.gridTemplateColumns.map(parseTrack);
    }
    if (css.gridTemplateRows) {
      grid.rows = css.gridTemplateRows.map(parseTrack);
    }
    if (css.columnGap !== undefined) {
      grid.columnGap = parseDim(css.columnGap);
    }
    if (css.rowGap !== undefined) {
      grid.rowGap = parseDim(css.rowGap);
    }
    node.display = { grid, type: "grid" };
  } else {
    const flex: FlexStyle = {};
    if (css.flexDirection !== undefined) {
      flex.direction = css.flexDirection;
    }
    if (css.flexWrap !== undefined) {
      flex.wrap = css.flexWrap;
    }
    if (css.justifyContent !== undefined) {
      flex.justify = css.justifyContent;
    }
    if (css.alignItems !== undefined) {
      flex.alignItems = css.alignItems;
    }
    if (css.flexGrow !== undefined) {
      flex.grow = css.flexGrow;
    }
    if (css.flexShrink !== undefined) {
      flex.shrink = css.flexShrink;
    }
    if (css.flexBasis !== undefined) {
      flex.basis = parseDim(css.flexBasis);
    }
    node.display = { flex, type: "flex" };
  }

  // Sizing
  if (css.width !== undefined) {
    node.width = parseDim(css.width);
  }
  if (css.height !== undefined) {
    node.height = parseDim(css.height);
  }
  if (css.minWidth !== undefined) {
    node.minWidth = parseDim(css.minWidth);
  }
  if (css.minHeight !== undefined) {
    node.minHeight = parseDim(css.minHeight);
  }
  if (css.maxWidth !== undefined) {
    node.maxWidth = parseDim(css.maxWidth);
  }
  if (css.maxHeight !== undefined) {
    node.maxHeight = parseDim(css.maxHeight);
  }

  // Padding (shorthand then individual overrides)
  if (css.padding !== undefined) {
    node.padding = expandSpacing(css.padding);
  }
  if (css.paddingTop !== undefined) {
    node.padding = node.padding ?? defaultSpacing();
    node.padding[0] = parseDim(css.paddingTop);
  }
  if (css.paddingRight !== undefined) {
    node.padding = node.padding ?? defaultSpacing();
    node.padding[1] = parseDim(css.paddingRight);
  }
  if (css.paddingBottom !== undefined) {
    node.padding = node.padding ?? defaultSpacing();
    node.padding[2] = parseDim(css.paddingBottom);
  }
  if (css.paddingLeft !== undefined) {
    node.padding = node.padding ?? defaultSpacing();
    node.padding[3] = parseDim(css.paddingLeft);
  }

  // Margin (shorthand then individual overrides)
  if (css.margin !== undefined) {
    node.margin = expandSpacing(css.margin);
  }
  if (css.marginTop !== undefined) {
    node.margin = node.margin ?? defaultSpacing();
    node.margin[0] = parseDim(css.marginTop);
  }
  if (css.marginRight !== undefined) {
    node.margin = node.margin ?? defaultSpacing();
    node.margin[1] = parseDim(css.marginRight);
  }
  if (css.marginBottom !== undefined) {
    node.margin = node.margin ?? defaultSpacing();
    node.margin[2] = parseDim(css.marginBottom);
  }
  if (css.marginLeft !== undefined) {
    node.margin = node.margin ?? defaultSpacing();
    node.margin[3] = parseDim(css.marginLeft);
  }

  // Gap
  if (css.gap !== undefined) {
    if (Array.isArray(css.gap)) {
      node.gap = [parseDim(css.gap[0]), parseDim(css.gap[1])];
    } else {
      const dim = parseDim(css.gap);
      node.gap = [dim, dim];
    }
  }

  // Border (passed through to Rust as visual style)
  if (css.border !== undefined) {
    (node as Record<string, unknown>).border = css.border;
  }
  if (css.borderRadius !== undefined) {
    (node as Record<string, unknown>).borderRadius = css.borderRadius;
  }
  if (css.borderColor !== undefined) {
    if (typeof css.borderColor === "string") {
      (node as Record<string, unknown>).borderColor = css.borderColor;
    } else {
      const c = css.borderColor;
      if (c.type === "rgb") {
        const HEX = 16;
        const PAD = 2;
        const r = c.r.toString(HEX).padStart(PAD, "0");
        const g = c.g.toString(HEX).padStart(PAD, "0");
        const b = c.b.toString(HEX).padStart(PAD, "0");
        (node as Record<string, unknown>).borderColor = `#${r}${g}${b}`;
      }
    }
  }

  // Text style
  text.fg = resolveColor(css.color);
  text.bg = resolveColor(css.backgroundColor);
  text.underlineColor = resolveColor(css.underlineColor);
  if (css.fontWeight === "bold") {
    text.bold = true;
  }
  if (css.fontStyle === "italic") {
    text.italic = true;
  }
  if (css.textDecoration === "underline") {
    text.underline = true;
  }
  if (css.textDecoration === "strikethrough" || css.textDecoration === "line-through") {
    text.strikethrough = true;
  }
  if (css.textDecoration === "overline") {
    text.overline = true;
  }
  if (css.dim) {
    text.dim = true;
  }
  if (css.underlineStyle !== undefined) {
    text.underlineStyle = css.underlineStyle;
  }
  if (css.textOverflow !== undefined) {
    text.textOverflow = css.textOverflow;
  }

  return { node, text };
};
