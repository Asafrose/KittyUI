import { describe, expect, test } from "bun:test";
import { nativeAvailable } from "@kittyui/core";
import { createApp, parseSgrMouse, type AppHandle, type AppOptions } from "./app.js";
import { TerminalContext, TerminalProvider, type TerminalContextValue } from "./context.js";

// ---------------------------------------------------------------------------
// Structural / type tests (no native lib required)
// ---------------------------------------------------------------------------

describe("createApp exports", () => {
  test("createApp is a function", () => {
    expect(typeof createApp).toBe("function");
  });

  test("AppHandle type has unmount and shutdown", () => {
    // Type-level check — we verify the shape via a type assertion.
    // At runtime we just confirm the exports exist.
    const handle: AppHandle = { unmount() {}, shutdown() {} };
    expect(handle.unmount).toBeFunction();
    expect(handle.shutdown).toBeFunction();
  });

  test("AppOptions accepts fps and debug", () => {
    const opts: AppOptions = { fps: 60, debug: true };
    expect(opts.fps).toBe(60);
    expect(opts.debug).toBe(true);
  });

  test("AppOptions is optional (all fields optional)", () => {
    const opts: AppOptions = {};
    expect(opts).toBeDefined();
  });
});

// ---------------------------------------------------------------------------
// SGR mouse sequence parser
// ---------------------------------------------------------------------------

describe("parseSgrMouse", () => {
  test("parses left button press", () => {
    const result = parseSgrMouse("[<0;10;20M");
    expect(result).not.toBeNull();
    expect(result!.button).toBe(0);
    expect(result!.col).toBe(9);  // 10 - 1 (1-based to 0-based)
    expect(result!.row).toBe(19); // 20 - 1
    expect(result!.isRelease).toBe(false);
  });

  test("parses button release (lowercase m)", () => {
    const result = parseSgrMouse("[<0;5;8m");
    expect(result).not.toBeNull();
    expect(result!.button).toBe(0);
    expect(result!.col).toBe(4);
    expect(result!.row).toBe(7);
    expect(result!.isRelease).toBe(true);
  });

  test("parses mouse move (button 35)", () => {
    const result = parseSgrMouse("[<35;50;30M");
    expect(result).not.toBeNull();
    expect(result!.button).toBe(35);
    expect(result!.col).toBe(49);
    expect(result!.row).toBe(29);
    expect(result!.isRelease).toBe(false);
  });

  test("parses scroll up (button 64)", () => {
    const result = parseSgrMouse("[<64;1;1M");
    expect(result).not.toBeNull();
    expect(result!.button).toBe(64);
    expect(result!.col).toBe(0);
    expect(result!.row).toBe(0);
  });

  test("parses scroll down (button 65)", () => {
    const result = parseSgrMouse("[<65;1;1M");
    expect(result).not.toBeNull();
    expect(result!.button).toBe(65);
  });

  test("returns null for non-mouse sequences", () => {
    expect(parseSgrMouse("[A")).toBeNull();       // arrow key
    expect(parseSgrMouse("hello")).toBeNull();         // plain text
    expect(parseSgrMouse("[<invalid")).toBeNull(); // malformed
    expect(parseSgrMouse("")).toBeNull();              // empty
  });

  test("handles large coordinates", () => {
    const result = parseSgrMouse("[<0;200;100M");
    expect(result).not.toBeNull();
    expect(result!.col).toBe(199);
    expect(result!.row).toBe(99);
  });
});

describe("TerminalContext exports", () => {
  test("TerminalContext is defined", () => {
    expect(TerminalContext).toBeDefined();
  });

  test("TerminalProvider is a function", () => {
    expect(typeof TerminalProvider).toBe("function");
  });

  test("TerminalContextValue shape", () => {
    const value: TerminalContextValue = {
      cols: 80,
      rows: 24,
      bridge: {} as TerminalContextValue["bridge"],
    };
    expect(value.cols).toBe(80);
    expect(value.rows).toBe(24);
    expect(value.bridge).toBeDefined();
  });
});

// ---------------------------------------------------------------------------
// Integration tests (require native lib)
// ---------------------------------------------------------------------------

describe.skipIf(!nativeAvailable)("createApp integration", () => {
  test("createApp returns handle with unmount and shutdown", () => {
    const React = require("react");
    const element = React.createElement("box", { style: { width: 10 } });

    // We need to capture process.exit to prevent the test runner from dying
    const originalExit = process.exit;
    let exitCalled = false;
    process.exit = (() => {
      exitCalled = true;
    }) as never;

    try {
      const handle = createApp(element);
      expect(handle).toBeDefined();
      expect(handle.unmount).toBeFunction();
      expect(handle.shutdown).toBeFunction();

      // Shutdown should not throw
      handle.shutdown();
      expect(exitCalled).toBe(true);
    } finally {
      process.exit = originalExit;
    }
  });

  test("shutdown is idempotent (calling twice does not throw)", () => {
    const React = require("react");
    const element = React.createElement("box");

    const originalExit = process.exit;
    process.exit = (() => {}) as never;

    try {
      const handle = createApp(element);
      handle.shutdown();
      // Second call should be a no-op
      handle.shutdown();
    } finally {
      process.exit = originalExit;
    }
  });

  test("createApp accepts custom fps option", () => {
    const React = require("react");
    const element = React.createElement("box");

    const originalExit = process.exit;
    process.exit = (() => {}) as never;

    try {
      const handle = createApp(element, { fps: 60 });
      expect(handle).toBeDefined();
      handle.shutdown();
    } finally {
      process.exit = originalExit;
    }
  });

  test("createApp accepts debug option", () => {
    const React = require("react");
    const element = React.createElement("box");

    const originalExit = process.exit;
    process.exit = (() => {}) as never;

    try {
      const handle = createApp(element, { debug: true });
      expect(handle).toBeDefined();
      handle.shutdown();
    } finally {
      process.exit = originalExit;
    }
  });
});
