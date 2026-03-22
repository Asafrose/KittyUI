/**
 * @kittyui/react — React bindings for KittyUI terminal rendering.
 */

export { hello } from "@kittyui/core";
export { createRoot, type KittyRoot } from "./reconciler.js";
export { BoxRenderable, ImageRenderable, TextRenderable, createRenderableForType, type KittyProps } from "./renderables.js";
export { createApp, type AppHandle, type AppOptions } from "./app.js";
export { EventDispatcher } from "./event-dispatcher.js";
export { TerminalContext, TerminalProvider, type TerminalContextValue } from "./context.js";

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
