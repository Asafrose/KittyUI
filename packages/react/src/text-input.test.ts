/**
 * Tests for the TextInput component.
 *
 * These tests verify exports, types, and basic invariants without
 * requiring the native Rust library.
 */

import { describe, expect, test } from "bun:test";
import { TextInput } from "./text-input.js";
import type { TextInputProps } from "./text-input.js";

describe("TextInput exports", () => {
  test("TextInput is a function", () => {
    expect(typeof TextInput).toBe("function");
  });

  test("TextInput is exported from index", async () => {
    const mod = await import("./index.js");
    expect(typeof mod.TextInput).toBe("function");
  });
});

describe("TextInputProps type", () => {
  test("TextInputProps has expected shape", () => {
    // Compile-time type check
    const props: TextInputProps = {
      value: "hello",
      onChange: (_v: string) => {},
    };
    expect(props.value).toBe("hello");
    expect(typeof props.onChange).toBe("function");
  });

  test("TextInputProps accepts optional placeholder", () => {
    const props: TextInputProps = {
      value: "",
      onChange: () => {},
      placeholder: "Type here...",
    };
    expect(props.placeholder).toBe("Type here...");
  });

  test("TextInputProps accepts optional style", () => {
    const props: TextInputProps = {
      value: "",
      onChange: () => {},
      style: { color: "red" },
    };
    expect(props.style).toBeDefined();
  });

  test("TextInputProps accepts optional autoFocus", () => {
    const props: TextInputProps = {
      value: "",
      onChange: () => {},
      autoFocus: true,
    };
    expect(props.autoFocus).toBe(true);
  });
});
