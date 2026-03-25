/**
 * EventDispatcher — dispatches events from the Rust engine to React component
 * event handlers (onClick, onFocus, onBlur, onKeyDown, etc.).
 *
 * Handles:
 * - Mouse events (click, mousedown, mouseup, mousemove, mouseenter, mouseleave)
 * - Keyboard events (keydown, keyup, keypress)
 * - Focus events (focus, blur)
 * - Event bubbling up the component tree
 */

import type { Bridge, RenderableTree } from "@kittyui/core";
import type {
  KittyEvent,
  MouseEvent as CoreMouseEvent,
  KeyboardEvent as CoreKeyboardEvent,
  FfiFocusEvent,
  FfiBlurEvent,
} from "@kittyui/core";
import type { BoxRenderable } from "./renderables.js";
import type {
  KittyMouseEvent,
  KittyKeyboardEvent,
  KittyFocusEvent,
} from "./types.js";

// ---------------------------------------------------------------------------
// Mouse button mapping: core event button byte -> mouse action
// ---------------------------------------------------------------------------

/** Mouse button byte values from the Rust engine. */
const MOUSE_BUTTON_LEFT = 0;
const MOUSE_BUTTON_MIDDLE = 1;
const MOUSE_BUTTON_RIGHT = 2;
const MOUSE_BUTTON_RELEASE = 3;
const MOUSE_BUTTON_MOVE = 35;
const MOUSE_BUTTON_SCROLL_UP = 64;
const MOUSE_BUTTON_SCROLL_DOWN = 65;

// Keyboard event type values from pushKeyEvent (third arg)
const KEY_EVENT_DOWN = 0;
const KEY_EVENT_UP = 1;
const KEY_EVENT_PRESS = 2;

// ---------------------------------------------------------------------------
// EventDispatcher
// ---------------------------------------------------------------------------

export class EventDispatcher {
  private bridge: Bridge;
  private tree: RenderableTree;

  /** The node ID currently under the mouse pointer (for enter/leave tracking). */
  private hoveredNodeId: number | null = null;

  /** The previously focused node ID (for focus change detection). */
  private lastFocusedNodeId: number | null = null;

  constructor(bridge: Bridge, tree: RenderableTree) {
    this.bridge = bridge;
    this.tree = tree;
  }

  // -----------------------------------------------------------------------
  // Public API
  // -----------------------------------------------------------------------

  /**
   * Handle a batch of events from the Rust engine.
   * Register this with bridge.onEvents().
   */
  handleEvents(events: KittyEvent[]): void {
    for (const event of events) {
      switch (event.type) {
        case "mouse":
          this.handleMouseEvent(event);
          break;
        case "keyboard":
          this.handleKeyboardEvent(event);
          break;
        case "focus":
          this.handleFocusEvent(event);
          break;
        case "blur":
          this.handleBlurEvent(event);
          break;
        // resize events are not dispatched to components
      }
    }
  }

  /**
   * Handle a keyboard event from stdin input.
   * Use this for events pushed via bridge.pushKeyEvent().
   */
  handleStdinKeyEvent(keyCode: number, modifiers: number, eventType: number): void {
    const focusedNodeId = this.bridge.getFocusedNode();

    const kittyEvent: KittyKeyboardEvent = {
      nativeEvent: {
        key: { type: "char", char: String.fromCharCode(keyCode) },
        modifiers: {
          shift: (modifiers & 1) !== 0,
          alt: (modifiers & 2) !== 0,
          ctrl: (modifiers & 4) !== 0,
          super: (modifiers & 8) !== 0,
        },
        eventType: eventType === KEY_EVENT_UP ? "release" : eventType === KEY_EVENT_PRESS ? "press" : "press",
      },
    };

    const handlerKey =
      eventType === KEY_EVENT_UP
        ? "onKeyUp"
        : eventType === KEY_EVENT_PRESS
          ? "onKeyPress"
          : "onKeyDown";

    if (focusedNodeId !== null) {
      this.dispatchKeyboardToNode(focusedNodeId, handlerKey, kittyEvent);
    }

    // Also broadcast as a raw KittyEvent so useKeyboard hook listeners fire.
    this.bridge.notifyEventListeners([{ type: "keyboard" as const, keyCode, modifiers, eventType }]);
  }

  /**
   * Handle a mouse event parsed from stdin SGR sequences.
   * Creates a synthetic mouse event and dispatches via hitTest.
   */
  handleMouseFromStdin(button: number, col: number, row: number, isRelease: boolean): void {
    const effectiveButton = isRelease ? MOUSE_BUTTON_RELEASE : button;

    const hitPath = this.bridge.hitTest(col, row);
    const targetNodeId = hitPath.length > 0 ? hitPath[0] : null;

    const nativeEvent: CoreMouseEvent = {
      type: "mouse",
      button: effectiveButton,
      x: col,
      y: row,
      pixelX: 0,
      pixelY: 0,
      modifiers: 0,
      nodeId: targetNodeId ?? 0xffffffff,
    };

    const kittyEvent: KittyMouseEvent = {
      x: col,
      y: row,
      nativeEvent,
    };

    // Track enter/leave
    this.updateHoverState(targetNodeId, kittyEvent);

    // Dispatch based on button
    if (effectiveButton === MOUSE_BUTTON_LEFT) {
      this.dispatchMouseAlongPath(hitPath, "onMouseDown", kittyEvent);
      this.dispatchMouseAlongPath(hitPath, "onClick", kittyEvent);
    } else if (effectiveButton === MOUSE_BUTTON_RELEASE) {
      this.dispatchMouseAlongPath(hitPath, "onMouseUp", kittyEvent);
    } else if (effectiveButton === MOUSE_BUTTON_MOVE) {
      this.dispatchMouseAlongPath(hitPath, "onMouseMove", kittyEvent);
    } else if (effectiveButton === MOUSE_BUTTON_MIDDLE || effectiveButton === MOUSE_BUTTON_RIGHT) {
      this.dispatchMouseAlongPath(hitPath, "onMouseDown", kittyEvent);
    } else if (effectiveButton === MOUSE_BUTTON_SCROLL_UP || effectiveButton === MOUSE_BUTTON_SCROLL_DOWN) {
      const deltaY = effectiveButton === MOUSE_BUTTON_SCROLL_UP ? -1 : 1;
      this.dispatchScrollAlongPath(hitPath, deltaY);
    }

    // Broadcast to event listeners so useMouse hook fires
    this.bridge.notifyEventListeners([nativeEvent]);
  }

  // -----------------------------------------------------------------------
  // Mouse event handling
  // -----------------------------------------------------------------------

  private handleMouseEvent(event: CoreMouseEvent): void {
    const { x, y, button, modifiers, nodeId } = event;

    // Build the hit path: use hitTest for the full ancestor chain
    const hitPath = this.bridge.hitTest(x, y);

    // If hitPath is empty and nodeId is provided, use nodeId as fallback
    const targetNodeId = hitPath.length > 0 ? hitPath[0] : (nodeId !== 0xffffffff ? nodeId : null);

    const nativeEvent: CoreMouseEvent = event;

    const kittyEvent: KittyMouseEvent = {
      x,
      y,
      nativeEvent,
    };

    // Track enter/leave
    this.updateHoverState(targetNodeId, kittyEvent);

    // Determine which handler to call based on button
    if (button === MOUSE_BUTTON_LEFT) {
      // Left click (press)
      this.dispatchMouseAlongPath(hitPath, "onMouseDown", kittyEvent);
      this.dispatchMouseAlongPath(hitPath, "onClick", kittyEvent);
    } else if (button === MOUSE_BUTTON_RELEASE) {
      this.dispatchMouseAlongPath(hitPath, "onMouseUp", kittyEvent);
    } else if (button === MOUSE_BUTTON_MOVE) {
      this.dispatchMouseAlongPath(hitPath, "onMouseMove", kittyEvent);
    } else if (button === MOUSE_BUTTON_MIDDLE || button === MOUSE_BUTTON_RIGHT) {
      this.dispatchMouseAlongPath(hitPath, "onMouseDown", kittyEvent);
    } else if (button === MOUSE_BUTTON_SCROLL_UP || button === MOUSE_BUTTON_SCROLL_DOWN) {
      const deltaY = button === MOUSE_BUTTON_SCROLL_UP ? -1 : 1;
      this.dispatchScrollAlongPath(hitPath, deltaY);
    }
  }

  /**
   * Dispatch a mouse event along the hit path (bubbling).
   * Tries the deepest node first; if it doesn't handle it, walks up.
   */
  private dispatchMouseAlongPath(
    hitPath: number[],
    handlerKey: "onClick" | "onMouseDown" | "onMouseUp" | "onMouseMove",
    event: KittyMouseEvent,
  ): void {
    for (const nodeId of hitPath) {
      const renderable = this.tree.get(nodeId);
      if (!renderable) continue;

      const handler = (renderable as BoxRenderable).eventHandlers?.[handlerKey];
      if (handler) {
        handler(event);
        return; // Event handled, stop bubbling
      }
    }
  }

  /**
   * Dispatch scroll events along the hit path (bubbling).
   */
  private dispatchScrollAlongPath(hitPath: number[], deltaY: number): void {
    for (const nodeId of hitPath) {
      const renderable = this.tree.get(nodeId);
      if (!renderable) continue;

      const handler = (renderable as BoxRenderable).eventHandlers?.onScroll;
      if (handler) {
        handler({ deltaX: 0, deltaY });
        return;
      }
    }
  }

  /**
   * Track mouse enter/leave by comparing the current hover target
   * with the previous one.
   */
  private updateHoverState(
    currentNodeId: number | null,
    event: KittyMouseEvent,
  ): void {
    if (currentNodeId === this.hoveredNodeId) return;

    // Fire onMouseLeave on the old node
    if (this.hoveredNodeId !== null) {
      const oldRenderable = this.tree.get(this.hoveredNodeId);
      if (oldRenderable) {
        const leaveHandler = (oldRenderable as BoxRenderable).eventHandlers?.onMouseLeave;
        if (leaveHandler) {
          leaveHandler(event);
        }
      }
    }

    // Fire onMouseEnter on the new node
    if (currentNodeId !== null) {
      const newRenderable = this.tree.get(currentNodeId);
      if (newRenderable) {
        const enterHandler = (newRenderable as BoxRenderable).eventHandlers?.onMouseEnter;
        if (enterHandler) {
          enterHandler(event);
        }
      }
    }

    this.hoveredNodeId = currentNodeId;
  }

  // -----------------------------------------------------------------------
  // Keyboard event handling
  // -----------------------------------------------------------------------

  private handleKeyboardEvent(event: CoreKeyboardEvent): void {
    const focusedNodeId = this.bridge.getFocusedNode();
    if (focusedNodeId === null) return;

    const kittyEvent: KittyKeyboardEvent = {
      nativeEvent: {
        key: { type: "char", char: String.fromCharCode(event.keyCode) },
        modifiers: {
          shift: (event.modifiers & 1) !== 0,
          alt: (event.modifiers & 2) !== 0,
          ctrl: (event.modifiers & 4) !== 0,
          super: (event.modifiers & 8) !== 0,
        },
        eventType:
          event.eventType === KEY_EVENT_UP
            ? "release"
            : event.eventType === KEY_EVENT_PRESS
              ? "press"
              : "press",
      },
    };

    const handlerKey =
      event.eventType === KEY_EVENT_UP
        ? "onKeyUp"
        : event.eventType === KEY_EVENT_PRESS
          ? "onKeyPress"
          : "onKeyDown";

    this.dispatchKeyboardToNode(focusedNodeId, handlerKey, kittyEvent);
  }

  /**
   * Dispatch a keyboard event to a node, bubbling up to ancestors.
   */
  private dispatchKeyboardToNode(
    nodeId: number,
    handlerKey: "onKeyDown" | "onKeyUp" | "onKeyPress",
    event: KittyKeyboardEvent,
  ): void {
    let currentId: number | null = nodeId;

    while (currentId !== null) {
      const renderable = this.tree.get(currentId);
      if (!renderable) break;

      const handler = (renderable as BoxRenderable).eventHandlers?.[handlerKey];
      if (handler) {
        handler(event);
        return; // Event handled, stop bubbling
      }

      // Walk up to parent
      const parent = this.tree.parent(currentId);
      currentId = parent ? parent.nodeId : null;
    }
  }

  // -----------------------------------------------------------------------
  // Focus event handling
  // -----------------------------------------------------------------------

  private handleFocusEvent(event: FfiFocusEvent): void {
    // If there was a previously focused node, fire onBlur
    if (this.lastFocusedNodeId !== null && this.lastFocusedNodeId !== event.nodeId) {
      this.fireFocusHandler(this.lastFocusedNodeId, "onBlur");
    }

    // Fire onFocus on the newly focused node
    this.fireFocusHandler(event.nodeId, "onFocus");
    this.lastFocusedNodeId = event.nodeId;
  }

  private handleBlurEvent(event: FfiBlurEvent): void {
    this.fireFocusHandler(event.nodeId, "onBlur");

    if (this.lastFocusedNodeId === event.nodeId) {
      this.lastFocusedNodeId = null;
    }
  }

  private fireFocusHandler(
    nodeId: number,
    handlerKey: "onFocus" | "onBlur",
  ): void {
    const renderable = this.tree.get(nodeId);
    if (!renderable) return;

    const focusEvent: KittyFocusEvent = { nodeId };
    const handler = (renderable as BoxRenderable).eventHandlers?.[handlerKey];
    if (handler) {
      handler(focusEvent);
    }
  }
}
