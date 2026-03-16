/**
 * @kittyui/core — Core TypeScript bindings for the KittyUI rendering engine.
 *
 * Native Rust functions are loaded via bun:ffi.
 */

import { lib } from "./ffi.js";

/**
 * Calls the native Rust `hello()` function and returns its string result.
 */
export function hello(): string {
  return String(lib.symbols.hello());
}
