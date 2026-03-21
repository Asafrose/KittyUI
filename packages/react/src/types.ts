/**
 * JSX intrinsic element types, style props, and event handler props for KittyUI.
 */

import type { CSSStyle, KeyEvent, MouseEvent as CoreMouseEvent } from "@kittyui/core";
import type { BoxRenderable, TextRenderable } from "./renderables.js";

// ---------------------------------------------------------------------------
// Event handler types
// ---------------------------------------------------------------------------

/** Mouse event delivered to a KittyUI element. */
export interface KittyMouseEvent {
  /** X coordinate relative to the element. */
  x: number;
  /** Y coordinate relative to the element. */
  y: number;
  /** The underlying core mouse event. */
  nativeEvent: CoreMouseEvent;
}

/** Keyboard event delivered to a KittyUI element. */
export interface KittyKeyboardEvent {
  /** The underlying core key event. */
  nativeEvent: KeyEvent;
}

/** Focus/blur event delivered to a KittyUI element. */
export interface KittyFocusEvent {
  /** The node ID that gained or lost focus. */
  nodeId: number;
}

/** Scroll event delivered to a KittyUI element. */
export interface KittyScrollEvent {
  /** Scroll delta X. */
  deltaX: number;
  /** Scroll delta Y. */
  deltaY: number;
}

// ---------------------------------------------------------------------------
// Event handler props
// ---------------------------------------------------------------------------

export interface MouseEventHandlers {
  onClick?: (event: KittyMouseEvent) => void;
  onMouseEnter?: (event: KittyMouseEvent) => void;
  onMouseLeave?: (event: KittyMouseEvent) => void;
  onMouseDown?: (event: KittyMouseEvent) => void;
  onMouseUp?: (event: KittyMouseEvent) => void;
  onMouseMove?: (event: KittyMouseEvent) => void;
  onScroll?: (event: KittyScrollEvent) => void;
}

export interface KeyboardEventHandlers {
  onKeyDown?: (event: KittyKeyboardEvent) => void;
  onKeyUp?: (event: KittyKeyboardEvent) => void;
  onKeyPress?: (event: KittyKeyboardEvent) => void;
}

export interface FocusEventHandlers {
  onFocus?: (event: KittyFocusEvent) => void;
  onBlur?: (event: KittyFocusEvent) => void;
}

// ---------------------------------------------------------------------------
// Focus props
// ---------------------------------------------------------------------------

export interface FocusProps {
  /** Tab order for keyboard navigation. */
  tabIndex?: number;
  /** Whether to auto-focus this element on mount. */
  autoFocus?: boolean;
}

// ---------------------------------------------------------------------------
// Ref support
// ---------------------------------------------------------------------------

/** Callback ref or React ref object. */
export type KittyRef<T> =
  | ((instance: T | null) => void)
  | { current: T | null };

// ---------------------------------------------------------------------------
// Component props
// ---------------------------------------------------------------------------

export interface BoxProps
  extends MouseEventHandlers,
    KeyboardEventHandlers,
    FocusEventHandlers,
    FocusProps {
  style?: CSSStyle;
  children?: React.ReactNode;
  key?: React.Key;
  ref?: KittyRef<BoxRenderable>;
}

export interface TextProps {
  style?: CSSStyle;
  children?: string;
  key?: React.Key;
  ref?: KittyRef<TextRenderable>;
}

export interface ImageProps extends FocusProps {
  src: string;
  width?: number;
  height?: number;
  style?: CSSStyle;
  key?: React.Key;
  ref?: KittyRef<BoxRenderable>;
}
