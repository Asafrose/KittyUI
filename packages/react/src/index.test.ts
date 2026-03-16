import { describe, expect, test } from "bun:test";
import { hello } from "./index.js";
import { nativeAvailable } from "@kittyui/core";

describe("@kittyui/react", () => {
  test("re-exports hello from @kittyui/core", () => {
    expect(hello).toBeDefined();
    expect(typeof hello).toBe("function");
  });

  test.skipIf(!nativeAvailable)("hello returns expected greeting", () => {
    const result = hello();
    expect(result).toBe("Hello from kittyui-core!");
  });
});
