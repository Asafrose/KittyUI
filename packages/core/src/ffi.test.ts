import { describe, expect, test } from "bun:test";
import { nativeAvailable, lib } from "./ffi.js";
import { hello } from "./index.js";

describe.skipIf(!nativeAvailable)("ffi integration", () => {
  test("hello returns greeting from Rust", () => {
    const result = hello();
    expect(result).toBe("Hello from kittyui-core!");
  });

  test("hello returns a string type", () => {
    const result = hello();
    expect(typeof result).toBe("string");
  });

  test("native library loads successfully", () => {
    expect(lib).not.toBeNull();
    expect(lib!.symbols).toBeDefined();
    expect(lib!.symbols.hello).toBeDefined();
  });

  test("calling hello multiple times returns consistent results", () => {
    const r1 = hello();
    const r2 = hello();
    expect(r1).toBe(r2);
  });
});
