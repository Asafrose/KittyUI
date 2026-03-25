/**
 * @kittyui/react — React bindings for KittyUI terminal rendering.
 */

export { hello } from "@kittyui/core";
export { createRoot, type KittyRoot } from "./reconciler.js";
export { BoxRenderable, ImageRenderable, TextRenderable, createRenderableForType, type KittyProps } from "./renderables.js";
export { createApp, parseSgrMouse, KEY_UP, KEY_DOWN, KEY_RIGHT, KEY_LEFT, type AppHandle, type AppOptions, type SgrMouseEvent } from "./app.js";
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

// Components
export { Box, Text, Image } from "./components.js";

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
