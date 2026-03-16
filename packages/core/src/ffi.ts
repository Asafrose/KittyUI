/**
 * FFI bindings — loads the native kittyui-core Rust library via bun:ffi.
 */
import { dlopen, FFIType, suffix } from "bun:ffi";
import { join } from "node:path";

const libPath = join(import.meta.dir, "..", "native", `libkittyui_core.${suffix}`);

const lib = dlopen(libPath, {
  hello: {
    args: [],
    returns: FFIType.cstring,
  },
});

/**
 * Calls the native Rust `hello()` function and returns its string result.
 */
export function hello(): string {
  return String(lib.symbols.hello());
}
