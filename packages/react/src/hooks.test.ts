/**
 * Tests for KittyUI React hooks.
 *
 * These tests verify hook exports, types, and basic invariants without
 * requiring the native Rust library or react-dom.
 */

import { describe, expect, test, mock } from "bun:test";
import {
  useTerminal,
  useFocus,
  useKeyboard,
  useMouse,
} from "./hooks.js";
import type { UseFocusResult, UseKeyboardOptions, UseMouseResult } from "./hooks.js";
import { TerminalContext, type TerminalContextValue } from "./context.js";

// ---------------------------------------------------------------------------
// Export validation
// ---------------------------------------------------------------------------

describe("hooks exports", () => {
  test("useTerminal is a function", () => {
    expect(typeof useTerminal).toBe("function");
  });

  test("useFocus is a function", () => {
    expect(typeof useFocus).toBe("function");
  });

  test("useKeyboard is a function", () => {
    expect(typeof useKeyboard).toBe("function");
  });

  test("useMouse is a function", () => {
    expect(typeof useMouse).toBe("function");
  });
});

// ---------------------------------------------------------------------------
// Type-level validation (compile-time checks)
// ---------------------------------------------------------------------------

describe("hook type signatures", () => {
  test("UseFocusResult has expected shape", () => {
    // This is a compile-time test — if it compiles, the types are correct
    const result: UseFocusResult = {
      isFocused: false,
      focus: () => {},
      blur: () => {},
    };
    expect(result.isFocused).toBe(false);
    expect(typeof result.focus).toBe("function");
    expect(typeof result.blur).toBe("function");
  });

  test("UseKeyboardOptions has expected shape", () => {
    const opts: UseKeyboardOptions = { global: true };
    expect(opts.global).toBe(true);

    const defaultOpts: UseKeyboardOptions = {};
    expect(defaultOpts.global).toBeUndefined();
  });

  test("UseMouseResult has expected shape", () => {
    const result: UseMouseResult = {
      isHovered: false,
      isPressed: false,
      position: null,
    };
    expect(result.isHovered).toBe(false);
    expect(result.isPressed).toBe(false);
    expect(result.position).toBeNull();

    const hoveredResult: UseMouseResult = {
      isHovered: true,
      isPressed: true,
      position: { x: 5, y: 3 },
    };
    expect(hoveredResult.position!.x).toBe(5);
    expect(hoveredResult.position!.y).toBe(3);
  });
});

// ---------------------------------------------------------------------------
// TerminalContext availability
// ---------------------------------------------------------------------------

describe("TerminalContext", () => {
  test("TerminalContext is exported and has Provider", () => {
    expect(TerminalContext).toBeDefined();
    expect(TerminalContext.Provider).toBeDefined();
  });

  test("TerminalContextValue type is compatible", () => {
    const mockBridge = {
      setFocusable: () => {},
      focus: () => true,
      blur: () => true,
      getFocusedNode: () => null,
      onEvents: () => {},
      hitTest: () => [],
      getLayout: () => ({ x: 0, y: 0, width: 10, height: 5 }),
    };

    // Verifies the shape expected by hooks
    const value = {
      cols: 80,
      rows: 24,
      bridge: mockBridge,
    };
    expect(value.cols).toBe(80);
    expect(value.rows).toBe(24);
    expect(value.bridge).toBeDefined();
  });
});

// ---------------------------------------------------------------------------
// Re-export from index
// ---------------------------------------------------------------------------

describe("index re-exports hooks", () => {
  test("all hooks are available from index", async () => {
    const indexModule = await import("./index.js");
    expect(typeof indexModule.useTerminal).toBe("function");
    expect(typeof indexModule.useFocus).toBe("function");
    expect(typeof indexModule.useKeyboard).toBe("function");
    expect(typeof indexModule.useMouse).toBe("function");
  });
});
