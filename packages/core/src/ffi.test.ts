import { describe, expect, test } from "bun:test";
import { hello } from "./index.js";
import { lib } from "./ffi.js";

describe("ffi integration", () => {
  test("hello returns greeting from Rust", () => {
    const result = hello();
    expect(result).toBe("Hello from kittyui-core!");
  });

  test("hello returns a string type", () => {
    const result = hello();
    expect(typeof result).toBe("string");
  });

  test("native library loads successfully", () => {
    expect(lib).toBeDefined();
    expect(lib.symbols).toBeDefined();
    expect(lib.symbols.hello).toBeDefined();
  });

  test("calling hello multiple times returns consistent results", () => {
    const r1 = hello();
    const r2 = hello();
    expect(r1).toBe(r2);
  });
});
