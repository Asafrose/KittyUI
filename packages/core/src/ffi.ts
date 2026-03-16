/**
 * Low-level FFI bridge — loads the native kittyui-core Rust library via bun:ffi.
 */
import { dlopen, FFIType, suffix } from "bun:ffi";
import { join } from "node:path";

const libPath = join(import.meta.dir, "..", "native", `libkittyui_core.${suffix}`);

export const lib = dlopen(libPath, {
  hello: {
    args: [],
    returns: FFIType.cstring,
  },
});
