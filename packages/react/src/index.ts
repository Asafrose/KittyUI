/**
 * @kittyui/react — React bindings for KittyUI terminal rendering.
 */

export { hello } from "@kittyui/core";
export { createRoot, type KittyRoot } from "./reconciler.js";
export { BoxRenderable, ImageRenderable, TextRenderable, createRenderableForType, type KittyProps } from "./renderables.js";

// JSX prop and event types
export type {
  BoxProps,
  ImageProps,
  TextProps,
  KittyMouseEvent,
  KittyKeyboardEvent,
  KittyFocusEvent,
  KittyScrollEvent,
  KittyRef,
  MouseEventHandlers,
  KeyboardEventHandlers,
  FocusEventHandlers,
  FocusProps,
} from "./types.js";
