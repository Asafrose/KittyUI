/**
 * @kittyui/core — Core TypeScript bindings for the KittyUI rendering engine.
 *
 * Native Rust functions are loaded via bun:ffi.
 */

export { nativeAvailable } from "./ffi.js";
import { lib } from "./ffi.js";

/**
 * Calls the native Rust `hello()` function and returns its string result.
 *
 * @throws if the native library is not available (not built yet).
 */
export function hello(): string {
  if (!lib) {
    throw new Error("Native library not available — run `bun run build:native` first");
  }
  return String(lib.symbols.hello());
}
