/**
 * Core TypeScript types matching the Rust structs in packages/core-rust/src/.
 *
 * These types are the canonical TS representations of the C ABI types
 * exchanged over the FFI bridge.
 */

// ---------------------------------------------------------------------------
// Color (mirrors core-rust/src/ansi.rs Color enum)
// ---------------------------------------------------------------------------

/** Standard ANSI color (0-7). */
export interface AnsiColor {
  type: "ansi";
  index: number;
}

/** Bright ANSI color (0-7, mapped to 8-15 in 256-color mode). */
export interface AnsiBrightColor {
  type: "ansi-bright";
  index: number;
}

/** 256-color palette index. */
export interface PaletteColor {
  type: "palette";
  index: number;
}

/** 24-bit true color. */
export interface RgbColor {
  type: "rgb";
  r: number;
  g: number;
  b: number;
}

/** A terminal color — discriminated union matching Rust's `Color` enum. */
export type Color = AnsiColor | AnsiBrightColor | PaletteColor | RgbColor;

// ---------------------------------------------------------------------------
// Underline style (mirrors core-rust/src/ansi.rs UnderlineStyle enum)
// ---------------------------------------------------------------------------

export type UnderlineStyle =
  | "none"
  | "single"
  | "double"
  | "curly"
  | "dotted"
  | "dashed";

// ---------------------------------------------------------------------------
// Text style (mirrors core-rust/src/ansi.rs Style struct)
// ---------------------------------------------------------------------------

/** Visual style attributes for a terminal cell. */
export interface TextStyle {
  fg?: Color | undefined;
  bg?: Color | undefined;
  underlineColor?: Color | undefined;
  underlineStyle?: UnderlineStyle | undefined;
  bold?: boolean | undefined;
  dim?: boolean | undefined;
  italic?: boolean | undefined;
  underline?: boolean | undefined;
  blink?: boolean | undefined;
  reverse?: boolean | undefined;
  strikethrough?: boolean | undefined;
  overline?: boolean | undefined;
}

// ---------------------------------------------------------------------------
// Cell (mirrors core-rust/src/cell.rs Cell struct)
// ---------------------------------------------------------------------------

/** A single terminal cell. */
export interface Cell {
  /** The character displayed in this cell. */
  ch: string;
  /** Visual style. */
  style: TextStyle;
}

// ---------------------------------------------------------------------------
// CellBuffer (mirrors core-rust/src/cell.rs CellBuffer struct)
// ---------------------------------------------------------------------------

/** A 2D grid of cells. */
export interface CellBuffer {
  width: number;
  height: number;
  cells: Cell[];
}

// ---------------------------------------------------------------------------
// Dimension (mirrors core-rust/src/layout.rs Dim enum)
// ---------------------------------------------------------------------------

export interface DimCells {
  type: "cells";
  value: number;
}

export interface DimPercent {
  type: "percent";
  value: number;
}

export interface DimAuto {
  type: "auto";
}

/** A dimension value in the KittyUI coordinate system. */
export type Dim = DimCells | DimPercent | DimAuto;

// ---------------------------------------------------------------------------
// Flex style (mirrors core-rust/src/layout.rs FlexStyle, FlexDir, Wrap)
// ---------------------------------------------------------------------------

export type FlexDirection = "row" | "column" | "row-reverse" | "column-reverse";
export type FlexWrap = "no-wrap" | "wrap" | "wrap-reverse";
export type JustifyContent =
  | "start"
  | "end"
  | "center"
  | "space-between"
  | "space-around"
  | "space-evenly";
export type AlignItems =
  | "start"
  | "end"
  | "center"
  | "baseline"
  | "stretch";

export interface FlexStyle {
  direction?: FlexDirection | undefined;
  wrap?: FlexWrap | undefined;
  justify?: JustifyContent | undefined;
  alignItems?: AlignItems | undefined;
  grow?: number | undefined;
  shrink?: number | undefined;
  basis?: Dim | undefined;
}

// ---------------------------------------------------------------------------
// Grid style (mirrors core-rust/src/layout.rs GridStyle, TrackDef)
// ---------------------------------------------------------------------------

export interface TrackDefCells {
  type: "cells";
  value: number;
}

export interface TrackDefFr {
  type: "fr";
  value: number;
}

export interface TrackDefPercent {
  type: "percent";
  value: number;
}

export interface TrackDefAuto {
  type: "auto";
}

export type TrackDef = TrackDefCells | TrackDefFr | TrackDefPercent | TrackDefAuto;

export interface GridStyle {
  columns?: TrackDef[] | undefined;
  rows?: TrackDef[] | undefined;
  columnGap?: Dim | undefined;
  rowGap?: Dim | undefined;
}

// ---------------------------------------------------------------------------
// Display mode (mirrors core-rust/src/layout.rs DisplayMode enum)
// ---------------------------------------------------------------------------

export interface DisplayFlex {
  type: "flex";
  flex?: FlexStyle | undefined;
}

export interface DisplayGrid {
  type: "grid";
  grid?: GridStyle | undefined;
}

export type DisplayMode = DisplayFlex | DisplayGrid;

// ---------------------------------------------------------------------------
// Node style (mirrors core-rust/src/layout.rs NodeStyle struct)
// ---------------------------------------------------------------------------

/** Complete style for a layout node. */
export interface NodeStyle {
  display?: DisplayMode | undefined;
  width?: Dim | undefined;
  height?: Dim | undefined;
  minWidth?: Dim | undefined;
  minHeight?: Dim | undefined;
  maxWidth?: Dim | undefined;
  maxHeight?: Dim | undefined;
  /** [top, right, bottom, left] */
  padding?: [Dim, Dim, Dim, Dim] | undefined;
  /** [top, right, bottom, left] */
  margin?: [Dim, Dim, Dim, Dim] | undefined;
  /** [column, row] */
  gap?: [Dim, Dim] | undefined;
}

// ---------------------------------------------------------------------------
// Computed layout (mirrors core-rust/src/layout.rs ComputedLayout struct)
// ---------------------------------------------------------------------------

/** Computed layout for a single node, in cell coordinates. */
export interface ComputedLayout {
  x: number;
  y: number;
  width: number;
  height: number;
}

// ---------------------------------------------------------------------------
// Key event types (mirrors core-rust/src/keyboard.rs)
// ---------------------------------------------------------------------------

export type Key =
  | { type: "char"; char: string }
  | { type: "f"; n: number }
  | { type: "up" }
  | { type: "down" }
  | { type: "left" }
  | { type: "right" }
  | { type: "home" }
  | { type: "end" }
  | { type: "page-up" }
  | { type: "page-down" }
  | { type: "insert" }
  | { type: "delete" }
  | { type: "enter" }
  | { type: "tab" }
  | { type: "backspace" }
  | { type: "escape" }
  | { type: "caps-lock" }
  | { type: "scroll-lock" }
  | { type: "num-lock" }
  | { type: "print-screen" }
  | { type: "pause" }
  | { type: "menu" }
  | { type: "unknown"; code: number };

export interface Modifiers {
  shift: boolean;
  alt: boolean;
  ctrl: boolean;
  super: boolean;
}

export type KeyEventType = "press" | "repeat" | "release";

/** Structured keyboard event matching Rust's KeyEvent. */
export interface KeyEvent {
  key: Key;
  modifiers: Modifiers;
  eventType: KeyEventType;
}

// ---------------------------------------------------------------------------
// Mouse event types (mirrors core-rust/src/mouse.rs)
// ---------------------------------------------------------------------------

export type MouseButton =
  | "left"
  | "middle"
  | "right"
  | "scroll-up"
  | "scroll-down"
  | "scroll-left"
  | "scroll-right"
  | "none";

export type MouseEventKind =
  | "press"
  | "release"
  | "move"
  | "drag";

/** Structured mouse event matching Rust's MouseEvent. */
export interface MouseEvent {
  button: MouseButton;
  kind: MouseEventKind;
  x: number;
  y: number;
  pixelX: number;
  pixelY: number;
  modifiers: Modifiers;
}

// ---------------------------------------------------------------------------
// Resize event
// ---------------------------------------------------------------------------

export interface ResizeEvent {
  cols: number;
  rows: number;
  pixelWidth: number;
  pixelHeight: number;
}

// ---------------------------------------------------------------------------
// Hit test (mirrors core-rust/src/hit_test.rs)
// ---------------------------------------------------------------------------

export interface HitNodeMeta {
  zIndex: number;
  clipsChildren: boolean;
  interactive: boolean;
}

/** Result of a hit test. */
export interface HitResult {
  /** The deepest node at the hit point. */
  target: number;
  /** Ancestor chain from target to root (for event bubbling). */
  path: number[];
}

// ---------------------------------------------------------------------------
// Focus (mirrors core-rust/src/focus.rs)
// ---------------------------------------------------------------------------

export interface FocusMeta {
  tabIndex: number;
}

export type FocusEvent =
  | { type: "focus"; nodeId: number }
  | { type: "blur"; nodeId: number };
