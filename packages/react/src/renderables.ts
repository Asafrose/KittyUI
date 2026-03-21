/**
 * Concrete Renderable subclasses for each JSX element type.
 */

import { type CSSStyle, Renderable } from "@kittyui/core";
import type {
  FocusEventHandlers,
  FocusProps,
  KeyboardEventHandlers,
  MouseEventHandlers,
} from "./types.js";

// ---------------------------------------------------------------------------
// Shared props interface
// ---------------------------------------------------------------------------

/** Props shared by all KittyUI JSX elements. */
export interface KittyProps
  extends MouseEventHandlers,
    KeyboardEventHandlers,
    FocusEventHandlers,
    FocusProps {
  children?: unknown;
  style?: CSSStyle;
  src?: string;
  width?: number;
  height?: number;
}

// ---------------------------------------------------------------------------
// Event handler storage
// ---------------------------------------------------------------------------

type EventHandlerKey = keyof MouseEventHandlers | keyof KeyboardEventHandlers | keyof FocusEventHandlers;

const EVENT_HANDLER_KEYS: EventHandlerKey[] = [
  "onClick",
  "onMouseEnter",
  "onMouseLeave",
  "onMouseDown",
  "onMouseUp",
  "onMouseMove",
  "onScroll",
  "onKeyDown",
  "onKeyUp",
  "onKeyPress",
  "onFocus",
  "onBlur",
];

// ---------------------------------------------------------------------------
// Renderables
// ---------------------------------------------------------------------------

/** A box container element (like a div). */
export class BoxRenderable extends Renderable {
  readonly type = "box";

  /** Stored event handlers for this element. */
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  eventHandlers: Partial<Record<EventHandlerKey, ((...args: any[]) => void) | undefined>> = {};

  /** Focus tab index. */
  tabIndex: number | undefined;

  /** Whether to auto-focus on mount. */
  autoFocus: boolean | undefined;

  applyProps(props: KittyProps): void {
    if (props.style) {
      this.setStyle(props.style);
    }
    for (const key of EVENT_HANDLER_KEYS) {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      this.eventHandlers[key] = (props as any)[key];
    }
    this.tabIndex = props.tabIndex;
    this.autoFocus = props.autoFocus;
  }
}

/** A text element that renders a string. */
export class TextRenderable extends Renderable {
  readonly type = "text";

  applyProps(props: KittyProps): void {
    if (props.style) {
      this.setStyle(props.style);
    }
  }
}

/** An image element that renders via the Kitty graphics protocol. */
export class ImageRenderable extends Renderable {
  readonly type = "image";

  /** Image source path or URL. */
  src: string | undefined;

  /** Requested display width in cells. */
  displayWidth: number | undefined;

  /** Requested display height in cells. */
  displayHeight: number | undefined;

  /** Focus tab index. */
  tabIndex: number | undefined;

  /** Whether to auto-focus on mount. */
  autoFocus: boolean | undefined;

  applyProps(props: KittyProps): void {
    if (props.style) {
      this.setStyle(props.style);
    }
    this.src = props.src;
    this.displayWidth = props.width;
    this.displayHeight = props.height;
    this.tabIndex = props.tabIndex;
    this.autoFocus = props.autoFocus;
  }
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

/** Map from JSX tag name to Renderable constructor. */
const TAG_MAP: Record<string, new () => BoxRenderable | TextRenderable | ImageRenderable> = {
  box: BoxRenderable,
  image: ImageRenderable,
  text: TextRenderable,
};

/** Create a Renderable instance from a JSX tag name. */
export const createRenderableForType = (type: string): BoxRenderable | TextRenderable | ImageRenderable => {
  const Ctor = TAG_MAP[type];
  if (!Ctor) {
    throw new Error(`Unknown KittyUI element type: "${type}"`);
  }
  return new Ctor();
};
