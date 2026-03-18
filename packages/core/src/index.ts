/**
 * @kittyui/core — Core TypeScript bindings for the KittyUI rendering engine.
 *
 * Native Rust functions are loaded via bun:ffi.
 */

import { lib } from "./ffi.js";

export { nativeAvailable } from "./ffi.js";

export { MutationEncoder } from "./mutation-encoder.js";
export { EventDecoder } from "./event-decoder.js";
export type {
  KittyEvent,
  KeyboardEvent,
  MouseEvent,
  ResizeEvent,
  FocusEvent as FfiFocusEvent,
  BlurEvent as FfiBlurEvent,
} from "./event-decoder.js";
export { Bridge } from "./bridge.js";
export type { InitResult, NodeLayout } from "./bridge.js";

// Core types (mirrors Rust structs)
export type {
  Color,
  AnsiColor,
  AnsiBrightColor,
  PaletteColor,
  RgbColor,
  UnderlineStyle,
  TextStyle,
  Cell,
  CellBuffer,
  Dim,
  DimCells,
  DimPercent,
  DimAuto,
  FlexDirection,
  FlexWrap,
  JustifyContent,
  AlignItems,
  FlexStyle,
  TrackDef,
  GridStyle,
  DisplayMode,
  DisplayFlex,
  DisplayGrid,
  NodeStyle,
  ComputedLayout,
  Key,
  Modifiers,
  KeyEventType,
  KeyEvent,
  MouseButton,
  MouseEventKind,
  MouseEvent as MouseEventType,
  ResizeEvent as ResizeEventType,
  HitNodeMeta,
  HitResult,
  FocusMeta,
  FocusEvent,
} from "./types.js";

// Color parsing
export { parseColor } from "./color.js";

// Style normalization
export type { CSSStyle, DimInput, GridTrackInput } from "./style.js";
export { normalizeStyle, parseDim } from "./style.js";

// Renderable base class
export { Renderable, resetNodeIdCounter } from "./renderable.js";

// RenderableTree
export { RenderableTree } from "./renderable-tree.js";

// BoxRenderable
export { BoxRenderable, resolveBorderChars } from "./box.js";
export type {
  BackgroundCell,
  BorderCell,
  BorderChars,
  BorderPreset,
  BoxShadow,
  BoxStyle,
  Overflow,
  ResolvedBoxShadow,
  ShadowCell,
} from "./box.js";

// TextRenderable
export {
  TextRenderable,
  alignLine,
  measureText,
  resolveSpans,
  truncateLine,
  wrapText,
} from "./text.js";
export type {
  AlignLineOptions,
  StyledChar,
  TextAlign,
  TextMeasurement,
  TextOptions,
  TextOverflow,
  TextSpan,
  TextWrap,
} from "./text.js";

/**
 * Calls the native Rust `hello()` function and returns its string result.
 *
 * @throws If the native library is not available (not built yet).
 */
export const hello = (): string => {
  if (!lib) {
    throw new Error("Native library not available — run `bun run build:native` first");
  }
  return String(lib.symbols.hello());
};
