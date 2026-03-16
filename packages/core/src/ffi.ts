/**
 * Low-level FFI bridge — loads the native kittyui-core Rust library via bun:ffi.
 */
import { FFIType, dlopen, suffix } from "bun:ffi";
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

const loadLib = () => {
  if (nativeAvailable) {
    return dlopen(libPath, symbols);
  }
  return undefined;
};

export const lib = loadLib();
