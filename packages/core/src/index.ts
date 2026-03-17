/**
 * @kittyui/core — Core TypeScript bindings for the KittyUI rendering engine.
 *
 * Native Rust functions are loaded via bun:ffi.
 */

import { lib } from "./ffi.js";

export { nativeAvailable } from "./ffi.js";

export { MutationEncoder } from "./mutation-encoder.js";
export { EventDecoder } from "./event-decoder.js";
export type { KittyEvent, KeyboardEvent, MouseEvent, ResizeEvent } from "./event-decoder.js";
export { Bridge } from "./bridge.js";
export type { InitResult, NodeLayout } from "./bridge.js";

/**
 * Calls the native Rust `hello()` function and returns its string result.
 *
 * @throws If the native library is not available (not built yet).
 */
export const hello = (): string => {
  if (!lib) {
    throw new Error("Native library not available — run `bun run build:native` first");
  }
  return String(lib.symbols.hello());
};
