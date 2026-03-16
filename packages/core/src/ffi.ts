/**
 * Low-level FFI bridge — loads the native kittyui-core Rust library via bun:ffi.
 */
import { dlopen, FFIType, suffix } from "bun:ffi";
import { existsSync } from "node:fs";
import { join } from "node:path";

const libPath = join(import.meta.dir, "..", "native", `libkittyui_core.${suffix}`);

export const nativeAvailable = existsSync(libPath);

const symbols = {
  hello: {
    args: [],
    returns: FFIType.cstring,
  },
} as const;

export const lib = nativeAvailable ? dlopen(libPath, symbols) : null;
