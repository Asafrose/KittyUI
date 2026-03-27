/**
 * TextInput component for KittyUI.
 *
 * A controlled text input that handles keyboard events for typing,
 * cursor movement, and deletion.
 */

import { createElement, useState, useRef, useEffect } from "react";
import type { CSSStyle } from "@kittyui/core";
import type { BoxRenderable } from "./renderables.js";
import { useFocus } from "./hooks.js";
import { useKeyboard } from "./hooks.js";
import { KEY_LEFT, KEY_RIGHT } from "./app.js";

// Key codes
const BACKSPACE = 127;
const BACKSPACE_ALT = 8;
const DELETE = 0x7f; // same as 127 on most terminals; we also check keyCode 46
const KEY_HOME = 0x1006;
const KEY_END = 0x1007;

export interface TextInputProps {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  style?: CSSStyle;
  autoFocus?: boolean;
}

export function TextInput(props: TextInputProps) {
  const { value, onChange, placeholder, style, autoFocus } = props;
  const [cursorPos, setCursorPos] = useState(value.length);
  const boxRef = useRef<BoxRenderable>(null);
  const { isFocused, focus } = useFocus(boxRef);

  // Keep cursor within bounds when value changes externally
  useEffect(() => {
    setCursorPos((prev: number) => Math.min(prev, value.length));
  }, [value]);

  // Auto-focus on mount if requested
  useEffect(() => {
    if (autoFocus) {
      focus();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useKeyboard(
    (event) => {
      if (!isFocused) return;

      const code = event.keyCode;

      // Arrow keys
      if (code === KEY_LEFT) {
        setCursorPos((prev: number) => Math.max(0, prev - 1));
        return;
      }
      if (code === KEY_RIGHT) {
        setCursorPos((prev: number) => Math.min(value.length, prev + 1));
        return;
      }

      // Home / End
      if (code === KEY_HOME) {
        setCursorPos(0);
        return;
      }
      if (code === KEY_END) {
        setCursorPos(value.length);
        return;
      }

      // Backspace
      if (code === BACKSPACE || code === BACKSPACE_ALT) {
        if (cursorPos > 0) {
          const next = value.slice(0, cursorPos - 1) + value.slice(cursorPos);
          setCursorPos(cursorPos - 1);
          onChange(next);
        }
        return;
      }

      // Delete (keyCode 46 on some terminals)
      if (code === 46) {
        if (cursorPos < value.length) {
          const next = value.slice(0, cursorPos) + value.slice(cursorPos + 1);
          onChange(next);
        }
        return;
      }

      // Printable characters (ASCII 32-126)
      if (code >= 32 && code <= 126) {
        const char = String.fromCharCode(code);
        const next = value.slice(0, cursorPos) + char + value.slice(cursorPos);
        setCursorPos(cursorPos + 1);
        onChange(next);
      }
    },
    { global: false },
  );

  // Build displayed text with cursor
  const showPlaceholder = value.length === 0 && !isFocused;
  const displayText = showPlaceholder ? (placeholder ?? "") : value;

  if (isFocused && !showPlaceholder) {
    // Render text with cursor: character at cursor position has inverse style
    const before = value.slice(0, cursorPos);
    const cursorChar = cursorPos < value.length ? value[cursorPos] : " ";
    const after = value.slice(cursorPos + 1);

    return createElement(
      "box",
      { ref: boxRef, style: { flexDirection: "row", ...style } },
      createElement("text", null, before),
      createElement(
        "text",
        { style: { textDecoration: "underline" as const, fontWeight: "bold" as const } },
        cursorChar,
      ),
      after.length > 0 ? createElement("text", null, after) : null,
    );
  }

  // Unfocused or placeholder
  const textStyle: CSSStyle | undefined = showPlaceholder
    ? { color: "#888888" }
    : undefined;

  return createElement(
    "box",
    { ref: boxRef, style },
    createElement("text", { style: textStyle }, displayText),
  );
}
