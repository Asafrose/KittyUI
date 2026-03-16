import { describe, expect, test } from "bun:test";
import { hello } from "./index.js";

describe("ffi", () => {
  test("hello returns greeting from Rust", () => {
    const result = hello();
    expect(result).toBe("Hello from kittyui-core!");
  });
});
