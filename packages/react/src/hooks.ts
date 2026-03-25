/**
 * Custom React hooks for KittyUI terminal applications.
 *
 * - useTerminal()  — terminal dimensions from context
 * - useFocus()     — focus management for a renderable
 * - useKeyboard()  — keyboard event listener
 * - useMouse()     — mouse hover/press tracking for a renderable
 */

import {
  useContext,
  useState,
  useEffect,
  useCallback,
  useRef,
  type RefObject,
} from "react";

import type {
  KittyEvent,
  KeyboardEvent as KittyKeyboardEvent,
} from "@kittyui/core";
import type { Renderable } from "@kittyui/core";

import { TerminalContext, type TerminalContextValue } from "./context.js";

// ---------------------------------------------------------------------------
// Internal helper
// ---------------------------------------------------------------------------

/**
 * Read TerminalContext and throw if used outside a provider.
 */
const useTerminalContext = (): TerminalContextValue => {
  const ctx = useContext(TerminalContext);
  if (!ctx) {
    throw new Error(
      "KittyUI hooks must be used within a <TerminalProvider>. " +
        "Wrap your app with createApp() or <TerminalProvider>.",
    );
  }
  return ctx;
};

// ---------------------------------------------------------------------------
// useTerminal
// ---------------------------------------------------------------------------

/**
 * Returns the current terminal dimensions.
 * Re-renders automatically when the terminal is resized.
 */
export const useTerminal = (): { cols: number; rows: number } => {
  const { cols, rows } = useTerminalContext();
  return { cols, rows };
};

// ---------------------------------------------------------------------------
// useFocus
// ---------------------------------------------------------------------------

export interface UseFocusResult {
  isFocused: boolean;
  focus: () => void;
  blur: () => void;
}

/**
 * Manages focus state for a renderable element.
 *
 * @param ref - React ref pointing to a Renderable (must have `nodeId`).
 */
export const useFocus = (
  ref: RefObject<Renderable | null>,
): UseFocusResult => {
  const { bridge } = useTerminalContext();
  const [isFocused, setIsFocused] = useState(false);

  // Register as focusable on mount and listen for focus/blur events.
  useEffect(() => {
    const nodeId = ref.current?.nodeId;
    if (nodeId === undefined) return;

    bridge.setFocusable(nodeId, true);

    let active = true;
    const listener = (events: KittyEvent[]): void => {
      if (!active) return;
      for (const event of events) {
        if (event.type === "focus" && event.nodeId === nodeId) {
          setIsFocused(true);
        } else if (event.type === "blur" && event.nodeId === nodeId) {
          setIsFocused(false);
        }
      }
    };
    bridge.onEvents(listener);

    return () => {
      active = false;
      if (ref.current?.nodeId !== undefined) {
        bridge.setFocusable(ref.current.nodeId, false);
      }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [bridge, ref]);

  const focus = useCallback(() => {
    const nodeId = ref.current?.nodeId;
    if (nodeId !== undefined) {
      bridge.focus(nodeId);
    }
  }, [bridge, ref]);

  const blur = useCallback(() => {
    bridge.blur();
  }, [bridge]);

  return { isFocused, focus, blur };
};

// ---------------------------------------------------------------------------
// useKeyboard
// ---------------------------------------------------------------------------

export interface UseKeyboardOptions {
  /**
   * When `true`, the handler fires for all keyboard events regardless of focus.
   * When `false` (default), events only fire when the component is focused.
   */
  global?: boolean;
}

/**
 * Registers a keyboard event listener.
 *
 * @param handler - Called for each keyboard event.
 * @param options - `{ global: true }` to receive all key events.
 */
export const useKeyboard = (
  handler: (event: KittyKeyboardEvent) => void,
  options?: UseKeyboardOptions,
): void => {
  const { bridge } = useTerminalContext();
  const handlerRef = useRef(handler);
  handlerRef.current = handler;

  const global = options?.global ?? false;

  useEffect(() => {
    let active = true;
    const listener = (events: KittyEvent[]): void => {
      if (!active) return;
      for (const event of events) {
        if (event.type === "keyboard") {
          if (global) {
            handlerRef.current(event);
          } else {
            // Non-global: only fire when there is a focused node.
            // The focused node check allows components using useFocus
            // to scope keyboard events to their focus state.
            const focused = bridge.getFocusedNode();
            if (focused !== null) {
              handlerRef.current(event);
            }
          }
        }
      }
    };
    bridge.onEvents(listener);

    return () => {
      active = false;
    };
  }, [bridge, global]);
};

// ---------------------------------------------------------------------------
// useMouse
// ---------------------------------------------------------------------------

export interface UseMouseResult {
  isHovered: boolean;
  isPressed: boolean;
  position: { x: number; y: number } | null;
}

/**
 * Tracks mouse hover and press state for a renderable element.
 *
 * @param ref - React ref pointing to a Renderable (must have `nodeId`).
 */
export const useMouse = (
  ref: RefObject<Renderable | null>,
): UseMouseResult => {
  const { bridge } = useTerminalContext();
  const [isHovered, setIsHovered] = useState(false);
  const [isPressed, setIsPressed] = useState(false);
  const [position, setPosition] = useState<{ x: number; y: number } | null>(
    null,
  );

  useEffect(() => {
    const nodeId = ref.current?.nodeId;
    if (nodeId === undefined) return;

    let active = true;
    const listener = (events: KittyEvent[]): void => {
      if (!active) return;
      for (const event of events) {
        if (event.type === "mouse") {
          // Check if the mouse is over this node using hit test
          const hitNodes = bridge.hitTest(event.x, event.y);
          const isOver = hitNodes.includes(nodeId);

          if (isOver) {
            // Get the node's layout to compute relative position
            const layout = bridge.getLayout(nodeId);
            setPosition({
              x: event.x - layout.x,
              y: event.y - layout.y,
            });
            setIsHovered(true);

            // Button 0 = left press, 1 = middle, 2 = right, 3 = release, 35 = move
            const RELEASE = 3;
            const MOVE = 35;
            if (event.button === RELEASE) {
              setIsPressed(false);
            } else if (event.button !== MOVE) {
              setIsPressed(true);
            }
          } else {
            if (isOver !== undefined) {
              setIsHovered(false);
              setIsPressed(false);
              setPosition(null);
            }
          }
        }
      }
    };
    bridge.onEvents(listener);

    return () => {
      active = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [bridge, ref]);

  return { isHovered, isPressed, position };
};
