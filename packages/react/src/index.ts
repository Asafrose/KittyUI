/**
 * @kittyui/react — React bindings for KittyUI terminal rendering.
 */

// Re-export JSX type augmentation so importing @kittyui/react registers
// <box>, <text>, <image> as intrinsic elements automatically.
export type {} from "./jsx.js";

export { hello } from "@kittyui/core";
export { createRoot, type KittyRoot } from "./reconciler.js";
export { BoxRenderable, ImageRenderable, TextRenderable, createRenderableForType, type KittyProps } from "./renderables.js";
export { createApp, KEY_UP, KEY_DOWN, KEY_RIGHT, KEY_LEFT, type AppHandle, type AppOptions } from "./app.js";
export { EventDispatcher } from "./event-dispatcher.js";
export { TerminalContext, TerminalProvider, type TerminalContextValue } from "./context.js";

// Hooks
export {
  useTerminal,
  useFocus,
  useKeyboard,
  useMouse,
  type UseFocusResult,
  type UseKeyboardOptions,
  type UseMouseResult,
} from "./hooks.js";

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
