/**
 * createApp — single-call entry point for KittyUI applications.
 *
 * Initialises the Rust engine, wires up the React reconciler, starts the
 * render loop, and handles stdin/resize/shutdown lifecycle.
 */

import { createElement, type ReactElement } from "react";
import { Bridge, MutationEncoder, RenderableTree } from "@kittyui/core";
import { createRoot } from "./reconciler.js";
import { TerminalProvider } from "./context.js";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const DEFAULT_FPS = 30;
const DEFAULT_COLS = 80;
const DEFAULT_ROWS = 24;
const MS_PER_SECOND = 1000;

// Key codes used for shutdown detection
const CTRL_C = "\x03";
const QUIT_KEY = "q";

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/** Options for `createApp()`. */
export interface AppOptions {
  /** Target frames per second for the render loop (default: 30). */
  fps?: number;
  /** Enable debug logging (default: false). */
  debug?: boolean;
}

/** Handle returned by `createApp()` for programmatic control. */
export interface AppHandle {
  /** Unmount the React tree (does not shut down the engine). */
  unmount(): void;
  /** Graceful shutdown: stops render loop, restores terminal, unmounts React, exits. */
  shutdown(): void;
}

// ---------------------------------------------------------------------------
// createApp
// ---------------------------------------------------------------------------

/**
 * Bootstrap a KittyUI terminal application from a single JSX element.
 *
 * ```tsx
 * import { createApp } from "@kittyui/react";
 * createApp(<App />);
 * ```
 */
export const createApp = (
  element: ReactElement,
  options: AppOptions = {},
): AppHandle => {
  const { fps = DEFAULT_FPS, debug = false } = options;

  // -----------------------------------------------------------------------
  // 1. Initialise Bridge (enters alt screen, hides cursor)
  // -----------------------------------------------------------------------
  const bridge = new Bridge();
  const initResult = bridge.init();

  if (debug) {
    const { versionMajor, versionMinor, versionPatch } = initResult;
    // eslint-disable-next-line no-console
    console.error(
      `[kittyui] engine v${versionMajor}.${versionMinor}.${versionPatch} initialised`,
    );
  }

  // -----------------------------------------------------------------------
  // 2. Create RenderableTree + MutationEncoder
  // -----------------------------------------------------------------------
  const encoder = bridge.getEncoder();
  const tree = new RenderableTree(encoder);

  // -----------------------------------------------------------------------
  // 3. Create React root via the reconciler
  // -----------------------------------------------------------------------
  const root = createRoot(tree);

  // -----------------------------------------------------------------------
  // 4. Render the user's element wrapped in TerminalProvider
  // -----------------------------------------------------------------------
  const wrappedElement = createElement(
    TerminalProvider,
    { bridge },
    element,
  );
  root.render(wrappedElement);

  // -----------------------------------------------------------------------
  // Track shutdown state
  // -----------------------------------------------------------------------
  let isShutdown = false;

  // -----------------------------------------------------------------------
  // 5. Set up stdin in raw mode for keyboard input
  // -----------------------------------------------------------------------
  const stdinDataHandler = (data: string | Buffer): void => {
    const key = typeof data === "string" ? data : data.toString("utf8");

    // Ctrl+C or 'q' triggers graceful shutdown
    if (key === CTRL_C || key === QUIT_KEY) {
      shutdown();
      return;
    }

    // Push every byte as a key event (keyCode = char code, no modifiers, type = keyDown=0)
    for (let i = 0; i < key.length; i++) {
      bridge.pushKeyEvent(key.charCodeAt(i), 0, 0);
    }
  };

  if (process.stdin.isTTY) {
    process.stdin.setRawMode(true);
    process.stdin.resume();
    process.stdin.setEncoding("utf8");
    process.stdin.on("data", stdinDataHandler);
  }

  // -----------------------------------------------------------------------
  // 6. Start render loop
  // -----------------------------------------------------------------------
  const FRAME_MS = Math.floor(MS_PER_SECOND / fps);
  const renderLoop = setInterval(() => {
    tree.flushDirtyStyles();
    bridge.flushMutations();
    bridge.renderFrame();
  }, FRAME_MS);

  // -----------------------------------------------------------------------
  // 7. Handle terminal resize
  // -----------------------------------------------------------------------
  const resizeHandler = (): void => {
    bridge.requestRender();
  };
  process.stdout.on("resize", resizeHandler);

  // -----------------------------------------------------------------------
  // 8. Handle SIGTERM / SIGINT
  // -----------------------------------------------------------------------
  const signalHandler = (): void => {
    shutdown();
  };
  process.on("SIGTERM", signalHandler);
  process.on("SIGINT", signalHandler);

  // -----------------------------------------------------------------------
  // Unmount (just React, not the engine)
  // -----------------------------------------------------------------------
  const unmount = (): void => {
    root.unmount();
  };

  // -----------------------------------------------------------------------
  // Graceful shutdown
  // -----------------------------------------------------------------------
  const shutdown = (): void => {
    if (isShutdown) return;
    isShutdown = true;

    // Stop render loop
    clearInterval(renderLoop);

    // Restore stdin
    if (process.stdin.isTTY) {
      process.stdin.off("data", stdinDataHandler);
      process.stdin.setRawMode(false);
      process.stdin.pause();
    }

    // Remove listeners
    process.stdout.off("resize", resizeHandler);
    process.off("SIGTERM", signalHandler);
    process.off("SIGINT", signalHandler);

    // Unmount React tree
    root.unmount();

    // Shut down Rust engine (exits alt screen, shows cursor)
    bridge.shutdown();

    if (debug) {
      // eslint-disable-next-line no-console
      console.error("[kittyui] shutdown complete");
    }

    // Exit process
    process.exit(0);
  };

  return { unmount, shutdown };
};
