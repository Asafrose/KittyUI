/**
 * EventSystem — orchestrates event dispatch, bubbling, delegation, and
 * hover state tracking for the renderable tree.
 *
 * The EventSystem sits at the root level and:
 * 1. Receives raw terminal events
 * 2. Performs hit-testing to find the target node
 * 3. Creates SyntheticEvents
 * 4. Bubbles events up the renderable tree (target -> root)
 * 5. Tracks hover state to emit mouseenter/mouseleave
 * 6. Supports event delegation via root-level handlers
 */

import { EventEmitter, type EventMap, type Listener } from "./event-emitter.js";
import { type AnySyntheticEvent, type MouseEventInit as MouseEventInitType, SyntheticFocusEvent, SyntheticKeyboardEvent, type SyntheticKeyboardEventType, SyntheticMouseEvent, type SyntheticMouseEventType, SyntheticResizeEvent } from "./synthetic-event.js";
import { type HitResult, type Key, type KeyEventType, type Modifiers, type MouseButton, type MouseEventKind } from "./types.js";
import { type Renderable } from "./renderable.js";
import { type RenderableTree } from "./renderable-tree.js";

// ---------------------------------------------------------------------------
// Event handler types
// ---------------------------------------------------------------------------

export type MouseHandler = (event: SyntheticMouseEvent) => void;
export type KeyboardHandler = (event: SyntheticKeyboardEvent) => void;
export type FocusHandler = (event: SyntheticFocusEvent) => void;
export type ResizeHandler = (event: SyntheticResizeEvent) => void;

// ---------------------------------------------------------------------------
// Node-level event map
// ---------------------------------------------------------------------------

export interface NodeEventMap extends EventMap {
  blur: SyntheticFocusEvent;
  click: SyntheticMouseEvent;
  focus: SyntheticFocusEvent;
  keydown: SyntheticKeyboardEvent;
  keypress: SyntheticKeyboardEvent;
  keyup: SyntheticKeyboardEvent;
  mousedown: SyntheticMouseEvent;
  mouseenter: SyntheticMouseEvent;
  mouseleave: SyntheticMouseEvent;
  mousemove: SyntheticMouseEvent;
  mouseup: SyntheticMouseEvent;
  resize: SyntheticResizeEvent;
}

// ---------------------------------------------------------------------------
// Hit test provider
// ---------------------------------------------------------------------------

/**
 * Function that performs a hit test at (col, row) and returns the target node
 * and its ancestor path for bubbling. If nothing is hit, returns undefined.
 */
export type HitTestFn = (col: number, row: number) => HitResult | undefined;

// ---------------------------------------------------------------------------
// Dispatch options
// ---------------------------------------------------------------------------

export interface DispatchMouseOptions {
  button: MouseButton;
  col: number;
  kind: MouseEventKind;
  modifiers: Modifiers;
  pixelX: number;
  pixelY: number;
  row: number;
}

export interface DispatchKeyboardOptions {
  eventType: KeyEventType;
  focusedNodeId: number;
  key: Key;
  modifiers: Modifiers;
}

export interface DispatchResizeOptions {
  cols: number;
  pixelHeight: number;
  pixelWidth: number;
  rows: number;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const ROOT_TARGET = 0;

type HoverBaseInit = Omit<MouseEventInitType, "target" | "type">;

interface BubblePath {
  path: number[];
  target: number;
}

// ---------------------------------------------------------------------------
// EventSystem
// ---------------------------------------------------------------------------

export class EventSystem {
  private tree: RenderableTree;
  private hitTest: HitTestFn;

  /** Per-node event emitters, keyed by node ID. */
  private nodeEmitters = new Map<number, EventEmitter<NodeEventMap>>();

  /** Root-level delegation handlers (receive all events before bubbling). */
  private delegationEmitter = new EventEmitter<NodeEventMap>();

  /** The node ID currently hovered (for mouseenter/mouseleave). */
  private hoveredNodeId: number | undefined;

  constructor(tree: RenderableTree, hitTest: HitTestFn) {
    this.tree = tree;
    this.hitTest = hitTest;
  }

  // -----------------------------------------------------------------------
  // Node-level event registration
  // -----------------------------------------------------------------------

  /**
   * Register a handler on a specific node.
   * Returns a dispose function.
   */
  on<EventKey extends keyof NodeEventMap>(
    nodeId: number,
    event: EventKey,
    handler: Listener<NodeEventMap[EventKey]>,
  ): () => void {
    let emitter = this.nodeEmitters.get(nodeId);
    if (!emitter) {
      emitter = new EventEmitter<NodeEventMap>();
      this.nodeEmitters.set(nodeId, emitter);
    }
    return emitter.on(event, handler);
  }

  /**
   * Remove a handler from a specific node.
   */
  off<EventKey extends keyof NodeEventMap>(
    nodeId: number,
    event: EventKey,
    handler: Listener<NodeEventMap[EventKey]>,
  ): void {
    const emitter = this.nodeEmitters.get(nodeId);
    if (emitter) {
      emitter.off(event, handler);
    }
  }

  // -----------------------------------------------------------------------
  // Root-level delegation
  // -----------------------------------------------------------------------

  /**
   * Register a delegation handler at the root level.
   * Delegation handlers fire for ALL events of a given type,
   * regardless of which node is targeted.
   */
  delegate<EventKey extends keyof NodeEventMap>(
    event: EventKey,
    handler: Listener<NodeEventMap[EventKey]>,
  ): () => void {
    return this.delegationEmitter.on(event, handler);
  }

  // -----------------------------------------------------------------------
  // Event dispatch — Mouse
  // -----------------------------------------------------------------------

  /**
   * Dispatch a raw mouse event through the system.
   * Performs hit-testing, creates synthetic events, handles hover tracking,
   * and bubbles events up the tree.
   */
  dispatchMouse(opts: DispatchMouseOptions): void {
    const hit = this.hitTest(opts.col, opts.row);
    const target = hit?.target ?? ROOT_TARGET;
    const path = hit?.path ?? [];

    const eventType = mouseKindToEventType(opts.kind, opts.button);

    const syntheticEvent = new SyntheticMouseEvent({
      button: opts.button,
      col: opts.col,
      kind: opts.kind,
      modifiers: opts.modifiers,
      pixelX: opts.pixelX,
      pixelY: opts.pixelY,
      row: opts.row,
      target,
      type: eventType,
    });

    // Hover tracking (mouseenter / mouseleave)
    if (opts.kind === "move") {
      this.updateHover(target, opts);
    }

    // Delegation (root-level)
    this.delegationEmitter.emit(eventType, syntheticEvent);

    // Bubble: target -> ancestors -> root
    this.bubble(eventType, syntheticEvent, { path, target });
  }

  // -----------------------------------------------------------------------
  // Event dispatch — Keyboard
  // -----------------------------------------------------------------------

  /**
   * Dispatch a keyboard event. Keyboard events target the focused node
   * (provided by the caller) and bubble up.
   */
  dispatchKeyboard(opts: DispatchKeyboardOptions): void {
    const synType = keyEventTypeToSyntheticType(opts.eventType);
    const target = opts.focusedNodeId;
    const path = this.buildPathToRoot(target);

    const syntheticEvent = new SyntheticKeyboardEvent({
      eventType: opts.eventType,
      key: opts.key,
      modifiers: opts.modifiers,
      target,
      type: synType,
    });

    this.delegationEmitter.emit(synType, syntheticEvent);
    this.bubble(synType, syntheticEvent, { path, target });
  }

  // -----------------------------------------------------------------------
  // Event dispatch — Focus
  // -----------------------------------------------------------------------

  /**
   * Dispatch a focus or blur event on a node (does not bubble).
   */
  dispatchFocus(type: "focus" | "blur", nodeId: number): void {
    const syntheticEvent = new SyntheticFocusEvent(type, nodeId);
    this.delegationEmitter.emit(type, syntheticEvent);
    // Focus events do not bubble — fire only on target
    const emitter = this.nodeEmitters.get(nodeId);
    if (emitter) {
      syntheticEvent.currentTarget = nodeId;
      emitter.emit(type, syntheticEvent);
    }
  }

  // -----------------------------------------------------------------------
  // Event dispatch — Resize
  // -----------------------------------------------------------------------

  /**
   * Dispatch a resize event (fires only at root delegation level).
   */
  dispatchResize(opts: DispatchResizeOptions): void {
    const syntheticEvent = new SyntheticResizeEvent({
      cols: opts.cols,
      pixelHeight: opts.pixelHeight,
      pixelWidth: opts.pixelWidth,
      rows: opts.rows,
      target: ROOT_TARGET,
    });
    this.delegationEmitter.emit("resize", syntheticEvent);
  }

  // -----------------------------------------------------------------------
  // Hover tracking
  // -----------------------------------------------------------------------

  /** Get the currently hovered node ID, or undefined if nothing is hovered. */
  get hoveredNode(): number | undefined {
    return this.hoveredNodeId;
  }

  // -----------------------------------------------------------------------
  // Cleanup
  // -----------------------------------------------------------------------

  /**
   * Remove all event handlers for a node (call when node is removed from tree).
   */
  removeNode(nodeId: number): void {
    this.nodeEmitters.delete(nodeId);
    if (this.hoveredNodeId === nodeId) {
      this.hoveredNodeId = undefined;
    }
  }

  /**
   * Remove all handlers and reset state.
   */
  dispose(): void {
    this.nodeEmitters.clear();
    this.delegationEmitter.removeAllListeners();
    this.hoveredNodeId = undefined;
  }

  // -----------------------------------------------------------------------
  // Internal — bubbling
  // -----------------------------------------------------------------------

  private bubble<EventKey extends keyof NodeEventMap>(
    eventType: EventKey,
    syntheticEvent: AnySyntheticEvent,
    bubblePath: BubblePath,
  ): void {
    this.fireOnNode(eventType, syntheticEvent, bubblePath.target);
    if (syntheticEvent.isPropagationStopped) {
      return;
    }

    for (const ancestorId of bubblePath.path) {
      if (ancestorId !== bubblePath.target) {
        this.fireOnNode(eventType, syntheticEvent, ancestorId);
        if (syntheticEvent.isPropagationStopped) {
          return;
        }
      }
    }
  }

  private fireOnNode<EventKey extends keyof NodeEventMap>(
    eventType: EventKey,
    syntheticEvent: AnySyntheticEvent,
    nodeId: number,
  ): void {
    const emitter = this.nodeEmitters.get(nodeId);
    if (emitter) {
      syntheticEvent.currentTarget = nodeId;
      emitter.emit(eventType, syntheticEvent as NodeEventMap[EventKey]);
    }
  }

  // -----------------------------------------------------------------------
  // Internal — hover tracking
  // -----------------------------------------------------------------------

  private updateHover(newTarget: number, mouseOpts: DispatchMouseOptions): void {
    const prevHovered = this.hoveredNodeId;

    if (prevHovered === newTarget) {
      return;
    }

    const baseInit = {
      button: mouseOpts.button,
      col: mouseOpts.col,
      kind: mouseOpts.kind,
      modifiers: mouseOpts.modifiers,
      pixelX: mouseOpts.pixelX,
      pixelY: mouseOpts.pixelY,
      row: mouseOpts.row,
    };

    this.emitLeaveEvent(prevHovered, baseInit);
    this.emitEnterEvent(newTarget, baseInit);

    this.hoveredNodeId = resolveHoveredNode(newTarget);
  }

  private emitLeaveEvent(
    prevHovered: number | undefined,
    baseInit: HoverBaseInit,
  ): void {
    if (prevHovered === undefined) {
      return;
    }
    const leaveEvent = new SyntheticMouseEvent({
      ...baseInit,
      target: prevHovered,
      type: "mouseleave",
    });
    this.delegationEmitter.emit("mouseleave", leaveEvent);
    const emitter = this.nodeEmitters.get(prevHovered);
    if (emitter) {
      leaveEvent.currentTarget = prevHovered;
      emitter.emit("mouseleave", leaveEvent);
    }
  }

  private emitEnterEvent(
    newTarget: number,
    baseInit: HoverBaseInit,
  ): void {
    if (newTarget === ROOT_TARGET) {
      return;
    }
    const enterEvent = new SyntheticMouseEvent({
      ...baseInit,
      target: newTarget,
      type: "mouseenter",
    });
    this.delegationEmitter.emit("mouseenter", enterEvent);
    const emitter = this.nodeEmitters.get(newTarget);
    if (emitter) {
      enterEvent.currentTarget = newTarget;
      emitter.emit("mouseenter", enterEvent);
    }
  }

  // -----------------------------------------------------------------------
  // Internal — path building
  // -----------------------------------------------------------------------

  /**
   * Build the ancestor path from a node to the root by walking the tree.
   * Used for keyboard events where we don't have a hit-test path.
   */
  private buildPathToRoot(nodeId: number): number[] {
    const path: number[] = [nodeId];
    let current: Renderable | undefined = this.tree.get(nodeId);
    while (current) {
      const parent = this.tree.parent(current.nodeId);
      if (!parent) {
        break;
      }
      path.push(parent.nodeId);
      current = parent;
    }
    return path;
  }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const mouseKindToEventType = (kind: MouseEventKind, button: MouseButton): SyntheticMouseEventType => {
  switch (kind) {
    case "press": {
      return "mousedown";
    }
    case "release": {
      if (button === "none") {
        return "click";
      }
      return "mouseup";
    }
    case "move": {
      return "mousemove";
    }
    case "drag": {
      return "mousemove";
    }
  }
};

const resolveHoveredNode = (newTarget: number): number | undefined => {
  if (newTarget === ROOT_TARGET) {
    return undefined;
  }
  return newTarget;
};

const keyEventTypeToSyntheticType = (eventType: KeyEventType): SyntheticKeyboardEventType => {
  switch (eventType) {
    case "press": {
      return "keydown";
    }
    case "repeat": {
      return "keypress";
    }
    case "release": {
      return "keyup";
    }
  }
};
