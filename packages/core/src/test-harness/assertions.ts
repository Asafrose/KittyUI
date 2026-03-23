/**
 * Custom bun:test matchers for VirtualScreen assertions.
 */

import { expect } from "bun:test";
import type { VirtualScreen } from "./virtual-screen.js";

// ---------------------------------------------------------------------------
// Matcher implementations
// ---------------------------------------------------------------------------

expect.extend({
  toContainText(screen: unknown, text: string) {
    const vs = screen as VirtualScreen;
    const pass = vs.containsText(text);
    return {
      message: () =>
        pass
          ? `Expected screen NOT to contain "${text}"`
          : `Expected screen to contain "${text}" but it was not found.\nScreen content:\n${vs.toString()}`,
      pass,
    };
  },

  toHaveBgColor(screen: unknown, row: number, col: number, color: string) {
    const actual = (screen as VirtualScreen).bgAt(row, col);
    const pass = actual === color;
    return {
      message: () =>
        pass
          ? `Expected cell (${row},${col}) NOT to have bg="${color}"`
          : `Expected cell (${row},${col}) to have bg="${color}" but got "${actual ?? "undefined"}"`,
      pass,
    };
  },

  toHaveFgColor(screen: unknown, row: number, col: number, color: string) {
    const actual = (screen as VirtualScreen).fgAt(row, col);
    const pass = actual === color;
    return {
      message: () =>
        pass
          ? `Expected cell (${row},${col}) NOT to have fg="${color}"`
          : `Expected cell (${row},${col}) to have fg="${color}" but got "${actual ?? "undefined"}"`,
      pass,
    };
  },

  toHaveTextAt(screen: unknown, row: number, col: number, text: string) {
    const vs = screen as VirtualScreen;
    let actual = "";
    for (let i = 0; i < text.length; i++) {
      actual += vs.textAt(row, col + i) ?? "";
    }
    const pass = actual === text;
    return {
      message: () =>
        pass
          ? `Expected cell (${row},${col}) NOT to have text "${text}"`
          : `Expected text "${text}" at (${row},${col}) but got "${actual}"`,
      pass,
    };
  },
});

// ---------------------------------------------------------------------------
// Type augmentation
// ---------------------------------------------------------------------------

declare module "bun:test" {
  interface Matchers<T> {
    toContainText(text: string): void;
    toHaveTextAt(row: number, col: number, text: string): void;
    toHaveBgColor(row: number, col: number, color: string): void;
    toHaveFgColor(row: number, col: number, color: string): void;
  }
}
