/**
 * BoxRenderable — the fundamental container element for KittyUI.
 *
 * Provides border rendering, padding, margin, background color fill,
 * overflow control, and box shadow — all mapped to Taffy layout styles.
 */

import type { Color, ComputedLayout, Dim } from "./types.js";
import type { CSSStyle } from "./style.js";
import { Renderable } from "./renderable.js";
import { parseColor } from "./color.js";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/* eslint-disable no-magic-numbers */
const MIN_BORDER_SIZE = 2;
const BORDER_INSET = 1;
const NO_INSET = 0;
const MIN_BORDER_PAD = 1;
const DEFAULT_SHADOW_OFFSET_X = 1;
const DEFAULT_SHADOW_OFFSET_Y = 1;
const ZERO_SHADOW = 0;
const PAD_TOP = 0;
const PAD_RIGHT = 1;
const PAD_BOTTOM = 2;
const PAD_LEFT = 3;
const MIN_CONTENT_SIZE = 0;
const ZERO_CELLS = 0;
/* eslint-enable no-magic-numbers */

const ZERO_DIM: Dim = { type: "cells", value: 0 };
const ONE_DIM: Dim = { type: "cells", value: 1 };
const DEFAULT_SHADOW_CHAR = "\u2591";

// ---------------------------------------------------------------------------
// Border types
// ---------------------------------------------------------------------------

/** Characters used to render a border. */
export interface BorderChars {
  bottomLeft: string;
  bottomRight: string;
  horizontal: string;
  topLeft: string;
  topRight: string;
  vertical: string;
}

/** Built-in border presets. */
export type BorderPreset = "single" | "double" | "rounded" | "bold";

/* eslint-disable sort-keys */
const BORDER_PRESETS: Record<BorderPreset, BorderChars> = {
  single: {
    bottomLeft: "\u2514",
    bottomRight: "\u2518",
    horizontal: "\u2500",
    topLeft: "\u250C",
    topRight: "\u2510",
    vertical: "\u2502",
  },
  double: {
    bottomLeft: "\u255A",
    bottomRight: "\u255D",
    horizontal: "\u2550",
    topLeft: "\u2554",
    topRight: "\u2557",
    vertical: "\u2551",
  },
  rounded: {
    bottomLeft: "\u2570",
    bottomRight: "\u256F",
    horizontal: "\u2500",
    topLeft: "\u256D",
    topRight: "\u256E",
    vertical: "\u2502",
  },
  bold: {
    bottomLeft: "\u2517",
    bottomRight: "\u251B",
    horizontal: "\u2501",
    topLeft: "\u250F",
    topRight: "\u2513",
    vertical: "\u2503",
  },
};
/* eslint-enable sort-keys */

/** Resolve a border preset name to its characters. */
export const resolveBorderChars = (preset: BorderPreset): BorderChars =>
  BORDER_PRESETS[preset];

// ---------------------------------------------------------------------------
// Overflow
// ---------------------------------------------------------------------------

/** Overflow behavior for box content. */
export type Overflow = "visible" | "hidden" | "scroll";

// ---------------------------------------------------------------------------
// Box shadow
// ---------------------------------------------------------------------------

/** Configuration for a character-based box shadow. */
export interface BoxShadow {
  /** Character used to render the shadow. Defaults to "░". */
  char?: string;
  /** Shadow color. */
  color?: string | Color;
  /** Horizontal offset in cells (positive = right). */
  offsetX?: number;
  /** Vertical offset in cells (positive = down). */
  offsetY?: number;
}

/** Resolved (all fields populated) box shadow. */
export interface ResolvedBoxShadow {
  char: string;
  color: Color | undefined;
  offsetX: number;
  offsetY: number;
}

// ---------------------------------------------------------------------------
// Box style (extends CSSStyle with box-specific properties)
// ---------------------------------------------------------------------------

/** Style properties specific to BoxRenderable. */
export interface BoxStyle extends CSSStyle {
  /** Background fill color (alias for backgroundColor). */
  background?: string | Color | undefined;
  /** Border preset or custom characters. `false`/`undefined` = no border. */
  border?: BorderPreset | BorderChars | false | undefined;
  /** Border color. */
  borderColor?: string | Color | undefined;
  /** Box shadow — CSS string (e.g. "0 4px 6px rgba(0,0,0,0.3)") or structured config. */
  boxShadow?: string | BoxShadow | undefined;
  /** Overflow behavior. Default: "visible". */
  overflow?: Overflow | undefined;
}

// ---------------------------------------------------------------------------
// Render cell types
// ---------------------------------------------------------------------------

/** A positioned border character. */
export interface BorderCell {
  char: string;
  col: number;
  row: number;
}

/** A positioned background cell. */
export interface BackgroundCell {
  col: number;
  row: number;
}

/** A positioned shadow cell. */
export interface ShadowCell {
  char: string;
  col: number;
  row: number;
}

// ---------------------------------------------------------------------------
// Internal parameter types (used to reduce function parameter counts)
// ---------------------------------------------------------------------------

interface BorderGridParams {
  bc: BorderChars;
  cells: BorderCell[];
  lastCol: number;
  lastRow: number;
}

interface ShadowGridParams {
  cells: ShadowCell[];
  char: string;
  height: number;
  offsetX: number;
  offsetY: number;
  width: number;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Resolve a color input (string | Color | undefined) to Color | undefined. */
const resolveColorInput = (input: string | Color | undefined): Color | undefined => {
  if (input === undefined) {
    return undefined;
  }
  if (typeof input === "string") {
    return parseColor(input);
  }
  return input;
};

/** Extract a cells value from a Dim, returning 0 for non-cells dims. */
const dimToCells = (dim: Dim | undefined): number => {
  if (dim && dim.type === "cells") {
    return dim.value;
  }
  return ZERO_CELLS;
};

/** Ensure a Dim is at least MIN_BORDER_PAD cells. */
const ensureMinPad = (dd: Dim): Dim => {
  if (dd.type === "cells" && dd.value < MIN_BORDER_PAD) {
    return ONE_DIM;
  }
  return dd;
};

/** Compute border inset based on whether border chars exist. */
const computeBorderInset = (borderChars: BorderChars | undefined): number => {
  if (borderChars !== undefined) {
    return BORDER_INSET;
  }
  return NO_INSET;
};

// ---------------------------------------------------------------------------
// BoxRenderable
// ---------------------------------------------------------------------------

export class BoxRenderable extends Renderable {
  // -- Border ---------------------------------------------------------------
  private _borderChars: BorderChars | undefined;
  private _borderColor: Color | undefined;

  // -- Background -----------------------------------------------------------
  private _backgroundColor: Color | undefined;

  // -- Overflow -------------------------------------------------------------
  private _overflow: Overflow = "visible";

  // -- Shadow ---------------------------------------------------------------
  private _shadow: ResolvedBoxShadow | undefined;

  // -----------------------------------------------------------------------
  // Construction helpers
  // -----------------------------------------------------------------------

  /**
   * Create a BoxRenderable and apply the given style in one step.
   */
  static create(style?: BoxStyle): BoxRenderable {
    const box = new BoxRenderable();
    if (style) {
      box.setBoxStyle(style);
    }
    return box;
  }

  // -----------------------------------------------------------------------
  // Style
  // -----------------------------------------------------------------------

  /** Apply a BoxStyle (CSSStyle + box-specific properties). */
  setBoxStyle(style: BoxStyle): void {
    const { border, borderColor, background, overflow, boxShadow, ...cssStyle } = style;

    // When boxShadow is a CSS string, pass it through to the Rust pixel renderer.
    if (typeof boxShadow === "string") {
      (cssStyle as Record<string, unknown>).boxShadow = boxShadow;
    }

    this.applyBaseStyle(cssStyle, background);
    this.setBorder(border);
    this.setBorderColor(borderColor);
    this._backgroundColor = resolveColorInput(cssStyle.backgroundColor ?? background);
    this.applyOverflowAndShadow(overflow, boxShadow);

    if (this._borderChars) {
      this.applyBorderPadding();
    }

    this.markDirty();
  }

  // -----------------------------------------------------------------------
  // Border
  // -----------------------------------------------------------------------

  /** Get the resolved border characters, or undefined if no border. */
  get borderChars(): BorderChars | undefined {
    return this._borderChars;
  }

  /** Get the border color. */
  get borderColor(): Color | undefined {
    return this._borderColor;
  }

  /** Set the border. Accepts a preset name, custom chars, or false to remove. */
  setBorder(border: BorderPreset | BorderChars | false | undefined): void {
    if (border === false || border === undefined) {
      this._borderChars = undefined;
    } else if (typeof border === "string") {
      this._borderChars = resolveBorderChars(border);
    } else {
      this._borderChars = border;
    }
    this.markDirty();
  }

  /** Set the border color. */
  setBorderColor(color: string | Color | undefined): void {
    this._borderColor = resolveColorInput(color);
    this.markDirty();
  }

  // -----------------------------------------------------------------------
  // Background
  // -----------------------------------------------------------------------

  /** Get the background color. */
  get backgroundColor(): Color | undefined {
    return this._backgroundColor;
  }

  /** Set the background color. */
  setBackgroundColor(color: string | Color | undefined): void {
    this._backgroundColor = resolveColorInput(color);
    this.markDirty();
  }

  // -----------------------------------------------------------------------
  // Overflow
  // -----------------------------------------------------------------------

  /** Get the overflow mode. */
  get overflow(): Overflow {
    return this._overflow;
  }

  /** Set the overflow mode. */
  setOverflow(overflow: Overflow): void {
    this._overflow = overflow;
    this.markDirty();
  }

  // -----------------------------------------------------------------------
  // Box shadow
  // -----------------------------------------------------------------------

  /** Get the resolved box shadow, or undefined if none. */
  get shadow(): ResolvedBoxShadow | undefined {
    return this._shadow;
  }

  /** Set the box shadow. Pass undefined to remove. */
  setBoxShadow(shadow: string | BoxShadow | undefined): void {
    if (shadow === undefined) {
      this._shadow = undefined;
    } else if (typeof shadow === "string") {
      // CSS string shadows are handled by the Rust pixel renderer;
      // no cell-based shadow on the TS side.
      this._shadow = undefined;
    } else {
      this._shadow = {
        char: shadow.char ?? DEFAULT_SHADOW_CHAR,
        color: resolveColorInput(shadow.color),
        offsetX: shadow.offsetX ?? DEFAULT_SHADOW_OFFSET_X,
        offsetY: shadow.offsetY ?? DEFAULT_SHADOW_OFFSET_Y,
      };
    }
    this.markDirty();
  }

  // -----------------------------------------------------------------------
  // Rendering helpers
  // -----------------------------------------------------------------------

  /**
   * Render the border characters for this box given its computed layout.
   * Returns an array of { col, row, char } positions relative to the box origin.
   */
  renderBorder(layout?: ComputedLayout): BorderCell[] {
    const rect = layout ?? this.layout;
    if (!rect || !this._borderChars) {
      return [];
    }

    const { width, height } = rect;
    if (width < MIN_BORDER_SIZE || height < MIN_BORDER_SIZE) {
      return [];
    }

    return this.buildBorderCells(width, height, this._borderChars);
  }

  /**
   * Render background fill cells (inside the border, if any).
   * Returns an array of { col, row } positions relative to the box origin.
   */
  renderBackground(layout?: ComputedLayout): BackgroundCell[] {
    const rect = layout ?? this.layout;
    if (!rect || !this._backgroundColor) {
      return [];
    }

    const { width, height } = rect;
    const inset = computeBorderInset(this._borderChars);

    const cells: BackgroundCell[] = [];
    for (let row = inset; row < height - inset; row++) {
      for (let col = inset; col < width - inset; col++) {
        cells.push({ col, row });
      }
    }
    return cells;
  }

  /**
   * Render box shadow cells.
   * Returns an array of { col, row, char } positions relative to the box origin.
   */
  renderShadow(layout?: ComputedLayout): ShadowCell[] {
    const rect = layout ?? this.layout;
    if (!rect || !this._shadow) {
      return [];
    }

    return this.buildShadowCells(rect.width, rect.height, this._shadow);
  }

  /**
   * Get the content area (inside borders and padding) for child layout.
   */
  getContentArea(layout?: ComputedLayout): ComputedLayout | undefined {
    const rect = layout ?? this.layout;
    if (!rect) {
      return undefined;
    }

    const borderOffset = computeBorderInset(this._borderChars);
    const { padding } = this.nodeStyle;

    return this.computeContentRect(rect, borderOffset, padding);
  }

  // -----------------------------------------------------------------------
  // Private
  // -----------------------------------------------------------------------

  private applyBaseStyle(cssStyle: CSSStyle, background: string | Color | undefined): void {
    if (background !== undefined && cssStyle.backgroundColor === undefined) {
      cssStyle.backgroundColor = background;
    }
    this.setStyle(cssStyle);
  }

  private applyOverflowAndShadow(
    overflow: Overflow | undefined,
    boxShadow: string | BoxShadow | undefined,
  ): void {
    if (overflow !== undefined) {
      this._overflow = overflow;
    }
    if (boxShadow !== undefined) {
      this.setBoxShadow(boxShadow);
    }
  }

  private buildBorderCells(width: number, height: number, bc: BorderChars): BorderCell[] {
    const cells: BorderCell[] = [];
    const lastCol = width - BORDER_INSET;
    const lastRow = height - BORDER_INSET;
    const params: BorderGridParams = { bc, cells, lastCol, lastRow };

    this.pushBorderCorners(params);
    this.pushBorderEdges(params);

    return cells;
  }

  private pushBorderCorners(params: BorderGridParams): void {
    const { bc, cells, lastCol, lastRow } = params;
    cells.push({ char: bc.topLeft, col: 0, row: 0 });
    cells.push({ char: bc.topRight, col: lastCol, row: 0 });
    cells.push({ char: bc.bottomLeft, col: 0, row: lastRow });
    cells.push({ char: bc.bottomRight, col: lastCol, row: lastRow });
  }

  private pushBorderEdges(params: BorderGridParams): void {
    const { bc, cells, lastCol, lastRow } = params;

    // Top and bottom edges
    for (let cc = BORDER_INSET; cc < lastCol; cc++) {
      cells.push({ char: bc.horizontal, col: cc, row: 0 });
      cells.push({ char: bc.horizontal, col: cc, row: lastRow });
    }

    // Left and right edges
    for (let rr = BORDER_INSET; rr < lastRow; rr++) {
      cells.push({ char: bc.vertical, col: 0, row: rr });
      cells.push({ char: bc.vertical, col: lastCol, row: rr });
    }
  }

  private buildShadowCells(
    width: number,
    height: number,
    shadow: ResolvedBoxShadow,
  ): ShadowCell[] {
    const { offsetX, offsetY, char } = shadow;

    if (offsetX === ZERO_SHADOW && offsetY === ZERO_SHADOW) {
      return [];
    }

    const cells: ShadowCell[] = [];
    const params: ShadowGridParams = { cells, char, height, offsetX, offsetY, width };
    this.pushBottomShadow(params);
    this.pushRightShadow(params);

    return cells;
  }

  private pushBottomShadow(params: ShadowGridParams): void {
    const { cells, char, height, offsetX, offsetY, width } = params;
    if (offsetY <= ZERO_SHADOW) {
      return;
    }
    for (let rr = height; rr < height + offsetY; rr++) {
      for (let cc = offsetX; cc < width + offsetX; cc++) {
        cells.push({ char, col: cc, row: rr });
      }
    }
  }

  private pushRightShadow(params: ShadowGridParams): void {
    const { cells, char, height, offsetX, offsetY, width } = params;
    if (offsetX <= ZERO_SHADOW) {
      return;
    }
    for (let rr = offsetY; rr < height; rr++) {
      for (let cc = width; cc < width + offsetX; cc++) {
        cells.push({ char, col: cc, row: rr });
      }
    }
  }

  private computeContentRect(
    rect: ComputedLayout,
    borderOffset: number,
    padding: [Dim, Dim, Dim, Dim] | undefined,
  ): ComputedLayout {
    const padTop = borderOffset + dimToCells(padding?.[PAD_TOP]);
    const padRight = borderOffset + dimToCells(padding?.[PAD_RIGHT]);
    const padBottom = borderOffset + dimToCells(padding?.[PAD_BOTTOM]);
    const padLeft = borderOffset + dimToCells(padding?.[PAD_LEFT]);

    /* eslint-disable id-length */
    return {
      height: Math.max(MIN_CONTENT_SIZE, rect.height - padTop - padBottom),
      width: Math.max(MIN_CONTENT_SIZE, rect.width - padLeft - padRight),
      x: rect.x + padLeft,
      y: rect.y + padTop,
    };
    /* eslint-enable id-length */
  }

  /**
   * When a border is present, ensure the layout has at least 1-cell
   * padding on each side to reserve space for the border characters.
   */
  private applyBorderPadding(): void {
    const current = this.nodeStyle;
    const pad = current.padding ?? [ZERO_DIM, ZERO_DIM, ZERO_DIM, ZERO_DIM];

    this.setNodeStyle({
      ...current,
      padding: [
        ensureMinPad(pad[PAD_TOP]),
        ensureMinPad(pad[PAD_RIGHT]),
        ensureMinPad(pad[PAD_BOTTOM]),
        ensureMinPad(pad[PAD_LEFT]),
      ],
    });
  }
}
