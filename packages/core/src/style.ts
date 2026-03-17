/**
 * Style normalization — accepts CSS-like shorthand properties and converts
 * them to the canonical NodeStyle type used by the Taffy layout engine.
 */

import type { Dim, NodeStyle, DisplayMode, FlexStyle, GridStyle } from "./types.js";
import { parseColor } from "./color.js";
import type { Color, TextStyle } from "./types.js";

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
  display?: "flex" | "grid" | undefined;

  // Flex properties
  flexDirection?: "row" | "column" | "row-reverse" | "column-reverse" | undefined;
  flexWrap?: "no-wrap" | "wrap" | "wrap-reverse" | undefined;
  justifyContent?:
    | "start"
    | "end"
    | "center"
    | "space-between"
    | "space-around"
    | "space-evenly"
    | undefined;
  alignItems?: "start" | "end" | "center" | "baseline" | "stretch" | undefined;
  flexGrow?: number | undefined;
  flexShrink?: number | undefined;
  flexBasis?: DimInput | undefined;

  // Grid properties
  gridTemplateColumns?: GridTrackInput[] | undefined;
  gridTemplateRows?: GridTrackInput[] | undefined;
  columnGap?: DimInput | undefined;
  rowGap?: DimInput | undefined;

  // Sizing
  width?: DimInput | undefined;
  height?: DimInput | undefined;
  minWidth?: DimInput | undefined;
  minHeight?: DimInput | undefined;
  maxWidth?: DimInput | undefined;
  maxHeight?: DimInput | undefined;

  // Shorthand spacing
  padding?: DimInput | [DimInput, DimInput] | [DimInput, DimInput, DimInput, DimInput] | undefined;
  paddingTop?: DimInput | undefined;
  paddingRight?: DimInput | undefined;
  paddingBottom?: DimInput | undefined;
  paddingLeft?: DimInput | undefined;

  margin?: DimInput | [DimInput, DimInput] | [DimInput, DimInput, DimInput, DimInput] | undefined;
  marginTop?: DimInput | undefined;
  marginRight?: DimInput | undefined;
  marginBottom?: DimInput | undefined;
  marginLeft?: DimInput | undefined;

  gap?: DimInput | [DimInput, DimInput] | undefined;

  // Text style properties
  color?: string | Color | undefined;
  backgroundColor?: string | Color | undefined;
  fontWeight?: "normal" | "bold" | undefined;
  fontStyle?: "normal" | "italic" | undefined;
  textDecoration?:
    | "none"
    | "underline"
    | "strikethrough"
    | "overline"
    | undefined;
  underlineStyle?:
    | "none"
    | "single"
    | "double"
    | "curly"
    | "dotted"
    | "dashed"
    | undefined;
  underlineColor?: string | Color | undefined;
}

/** Input for a dimension value. */
export type DimInput = number | string | "auto";

/** Input for a grid track definition. */
export type GridTrackInput = number | string | "auto";

// ---------------------------------------------------------------------------
// Normalization
// ---------------------------------------------------------------------------

/** Convert a DimInput to a Dim. */
export function parseDim(input: DimInput): Dim {
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
}

/** Expand a shorthand spacing value into [top, right, bottom, left]. */
function expandSpacing(
  input: DimInput | [DimInput, DimInput] | [DimInput, DimInput, DimInput, DimInput],
): [Dim, Dim, Dim, Dim] {
  if (Array.isArray(input)) {
    if (input.length === 2) {
      const v = parseDim(input[0]);
      const h = parseDim(input[1]);
      return [v, h, v, h];
    }
    return [parseDim(input[0]), parseDim(input[1]), parseDim(input[2]), parseDim(input[3])];
  }
  const d = parseDim(input);
  return [d, d, d, d];
}

/** Parse a grid track input into a TrackDef. */
function parseTrack(input: GridTrackInput): import("./types.js").TrackDef {
  if (input === "auto") return { type: "auto" };
  if (typeof input === "number") return { type: "cells", value: input };
  const s = input.trim();
  if (s === "auto") return { type: "auto" };
  if (s.endsWith("fr")) {
    const v = Number.parseFloat(s);
    if (!Number.isNaN(v)) return { type: "fr", value: v };
  }
  if (s.endsWith("%")) {
    const v = Number.parseFloat(s);
    if (!Number.isNaN(v)) return { type: "percent", value: v };
  }
  const n = Number.parseFloat(s);
  if (!Number.isNaN(n)) return { type: "cells", value: n };
  return { type: "auto" };
}

/** Resolve a color input (string or Color object) to a Color. */
function resolveColor(input: string | Color | undefined): Color | undefined {
  if (input === undefined) return undefined;
  if (typeof input === "string") return parseColor(input);
  return input;
}

/**
 * Normalize a CSS-like style object into the canonical NodeStyle + TextStyle.
 */
export function normalizeStyle(css: CSSStyle): { node: NodeStyle; text: TextStyle } {
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
    if (css.columnGap !== undefined) grid.columnGap = parseDim(css.columnGap);
    if (css.rowGap !== undefined) grid.rowGap = parseDim(css.rowGap);
    node.display = { type: "grid", grid };
  } else {
    const flex: FlexStyle = {};
    if (css.flexDirection !== undefined) flex.direction = css.flexDirection;
    if (css.flexWrap !== undefined) flex.wrap = css.flexWrap;
    if (css.justifyContent !== undefined) flex.justify = css.justifyContent;
    if (css.alignItems !== undefined) flex.alignItems = css.alignItems;
    if (css.flexGrow !== undefined) flex.grow = css.flexGrow;
    if (css.flexShrink !== undefined) flex.shrink = css.flexShrink;
    if (css.flexBasis !== undefined) flex.basis = parseDim(css.flexBasis);
    node.display = { type: "flex", flex };
  }

  // Sizing
  if (css.width !== undefined) node.width = parseDim(css.width);
  if (css.height !== undefined) node.height = parseDim(css.height);
  if (css.minWidth !== undefined) node.minWidth = parseDim(css.minWidth);
  if (css.minHeight !== undefined) node.minHeight = parseDim(css.minHeight);
  if (css.maxWidth !== undefined) node.maxWidth = parseDim(css.maxWidth);
  if (css.maxHeight !== undefined) node.maxHeight = parseDim(css.maxHeight);

  // Padding (shorthand then individual overrides)
  if (css.padding !== undefined) {
    node.padding = expandSpacing(css.padding);
  }
  if (css.paddingTop !== undefined) {
    node.padding = node.padding ?? [
      { type: "cells", value: 0 },
      { type: "cells", value: 0 },
      { type: "cells", value: 0 },
      { type: "cells", value: 0 },
    ];
    node.padding[0] = parseDim(css.paddingTop);
  }
  if (css.paddingRight !== undefined) {
    node.padding = node.padding ?? [
      { type: "cells", value: 0 },
      { type: "cells", value: 0 },
      { type: "cells", value: 0 },
      { type: "cells", value: 0 },
    ];
    node.padding[1] = parseDim(css.paddingRight);
  }
  if (css.paddingBottom !== undefined) {
    node.padding = node.padding ?? [
      { type: "cells", value: 0 },
      { type: "cells", value: 0 },
      { type: "cells", value: 0 },
      { type: "cells", value: 0 },
    ];
    node.padding[2] = parseDim(css.paddingBottom);
  }
  if (css.paddingLeft !== undefined) {
    node.padding = node.padding ?? [
      { type: "cells", value: 0 },
      { type: "cells", value: 0 },
      { type: "cells", value: 0 },
      { type: "cells", value: 0 },
    ];
    node.padding[3] = parseDim(css.paddingLeft);
  }

  // Margin (shorthand then individual overrides)
  if (css.margin !== undefined) {
    node.margin = expandSpacing(css.margin);
  }
  if (css.marginTop !== undefined) {
    node.margin = node.margin ?? [
      { type: "cells", value: 0 },
      { type: "cells", value: 0 },
      { type: "cells", value: 0 },
      { type: "cells", value: 0 },
    ];
    node.margin[0] = parseDim(css.marginTop);
  }
  if (css.marginRight !== undefined) {
    node.margin = node.margin ?? [
      { type: "cells", value: 0 },
      { type: "cells", value: 0 },
      { type: "cells", value: 0 },
      { type: "cells", value: 0 },
    ];
    node.margin[1] = parseDim(css.marginRight);
  }
  if (css.marginBottom !== undefined) {
    node.margin = node.margin ?? [
      { type: "cells", value: 0 },
      { type: "cells", value: 0 },
      { type: "cells", value: 0 },
      { type: "cells", value: 0 },
    ];
    node.margin[2] = parseDim(css.marginBottom);
  }
  if (css.marginLeft !== undefined) {
    node.margin = node.margin ?? [
      { type: "cells", value: 0 },
      { type: "cells", value: 0 },
      { type: "cells", value: 0 },
      { type: "cells", value: 0 },
    ];
    node.margin[3] = parseDim(css.marginLeft);
  }

  // Gap
  if (css.gap !== undefined) {
    if (Array.isArray(css.gap)) {
      node.gap = [parseDim(css.gap[0]), parseDim(css.gap[1])];
    } else {
      const d = parseDim(css.gap);
      node.gap = [d, d];
    }
  }

  // Text style
  text.fg = resolveColor(css.color);
  text.bg = resolveColor(css.backgroundColor);
  text.underlineColor = resolveColor(css.underlineColor);
  if (css.fontWeight === "bold") text.bold = true;
  if (css.fontStyle === "italic") text.italic = true;
  if (css.textDecoration === "underline") text.underline = true;
  if (css.textDecoration === "strikethrough") text.strikethrough = true;
  if (css.textDecoration === "overline") text.overline = true;
  if (css.underlineStyle !== undefined) text.underlineStyle = css.underlineStyle;

  return { node, text };
}
