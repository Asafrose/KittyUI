import { describe, expect, test } from "bun:test";
import { nativeAvailable } from "@kittyui/core";
import { createApp, type AppHandle, type AppOptions } from "./app.js";
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
