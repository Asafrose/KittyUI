/**
 * SyntheticEvent — React-style synthetic events for the terminal.
 *
 * Wraps raw terminal events with:
 * - stopPropagation() / isPropagationStopped
 * - preventDefault() / isDefaultPrevented
 * - target / currentTarget node IDs
 * - event type and timestamp
 */

import type { Key, KeyEventType, Modifiers, MouseButton, MouseEventKind } from "./types.js";

// ---------------------------------------------------------------------------
// Base synthetic event
// ---------------------------------------------------------------------------

export abstract class SyntheticEvent<EventType extends string = string> {
  readonly type: EventType;
  readonly timestamp: number;

  /** The node ID where the event originated. */
  readonly target: number;

  /** The node ID of the current handler (set during bubbling). */
  currentTarget: number;

  private _propagationStopped = false;
  private _defaultPrevented = false;

  constructor(type: EventType, target: number) {
    this.type = type;
    this.timestamp = Date.now();
    this.target = target;
    this.currentTarget = target;
  }

  stopPropagation(): void {
    this._propagationStopped = true;
  }

  get isPropagationStopped(): boolean {
    return this._propagationStopped;
  }

  preventDefault(): void {
    this._defaultPrevented = true;
  }

  get isDefaultPrevented(): boolean {
    return this._defaultPrevented;
  }
}

// ---------------------------------------------------------------------------
// Keyboard synthetic event
// ---------------------------------------------------------------------------

export type SyntheticKeyboardEventType = "keydown" | "keyup" | "keypress";

export interface KeyboardEventInit {
  eventType: KeyEventType;
  key: Key;
  modifiers: Modifiers;
  target: number;
  type: SyntheticKeyboardEventType;
}

export class SyntheticKeyboardEvent extends SyntheticEvent<SyntheticKeyboardEventType> {
  readonly key: Key;
  readonly modifiers: Modifiers;
  readonly eventType: KeyEventType;

  constructor(init: KeyboardEventInit) {
    super(init.type, init.target);
    this.key = init.key;
    this.modifiers = init.modifiers;
    this.eventType = init.eventType;
  }
}

// ---------------------------------------------------------------------------
// Mouse synthetic event
// ---------------------------------------------------------------------------

export type SyntheticMouseEventType =
  | "mousedown"
  | "mouseup"
  | "mousemove"
  | "click"
  | "mouseenter"
  | "mouseleave";

export interface MouseEventInit {
  button: MouseButton;
  col: number;
  kind: MouseEventKind;
  modifiers: Modifiers;
  pixelX: number;
  pixelY: number;
  row: number;
  target: number;
  type: SyntheticMouseEventType;
}

export class SyntheticMouseEvent extends SyntheticEvent<SyntheticMouseEventType> {
  readonly button: MouseButton;
  readonly kind: MouseEventKind;
  readonly col: number;
  readonly row: number;
  readonly pixelX: number;
  readonly pixelY: number;
  readonly modifiers: Modifiers;

  constructor(init: MouseEventInit) {
    super(init.type, init.target);
    this.button = init.button;
    this.kind = init.kind;
    this.col = init.col;
    this.row = init.row;
    this.pixelX = init.pixelX;
    this.pixelY = init.pixelY;
    this.modifiers = init.modifiers;
  }
}

// ---------------------------------------------------------------------------
// Resize synthetic event
// ---------------------------------------------------------------------------

export interface ResizeEventInit {
  cols: number;
  pixelHeight: number;
  pixelWidth: number;
  rows: number;
  target: number;
}

export class SyntheticResizeEvent extends SyntheticEvent<"resize"> {
  readonly cols: number;
  readonly rows: number;
  readonly pixelWidth: number;
  readonly pixelHeight: number;

  constructor(init: ResizeEventInit) {
    super("resize", init.target);
    this.cols = init.cols;
    this.rows = init.rows;
    this.pixelWidth = init.pixelWidth;
    this.pixelHeight = init.pixelHeight;
  }
}

// ---------------------------------------------------------------------------
// Focus synthetic events
// ---------------------------------------------------------------------------

export class SyntheticFocusEvent extends SyntheticEvent<"focus" | "blur"> {}

// ---------------------------------------------------------------------------
// Union type
// ---------------------------------------------------------------------------

export type AnySyntheticEvent =
  | SyntheticKeyboardEvent
  | SyntheticMouseEvent
  | SyntheticResizeEvent
  | SyntheticFocusEvent;
