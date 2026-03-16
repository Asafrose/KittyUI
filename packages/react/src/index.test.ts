import { describe, expect, test } from "bun:test";
import { nativeAvailable } from "@kittyui/core";
import { hello } from "./index.js";

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
