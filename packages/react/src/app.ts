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
import { EventDispatcher } from "./event-dispatcher.js";

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

// Terminal mouse tracking escape sequences (SGR mode)
const MOUSE_ENABLE = "\x1b[?1000h\x1b[?1006h\x1b[?1003h";
const MOUSE_DISABLE = "\x1b[?1003l\x1b[?1006l\x1b[?1000l";

// SGR mouse sequence regex
// oxlint-disable-next-line no-control-regex
const SGR_MOUSE_RE = /^\x1b\[<(\d+);(\d+);(\d+)([Mm])$/;

// CSI t size response regex (pixel size / cell count replies)
// oxlint-disable-next-line no-control-regex
const CSI_SIZE_RE = /^\x1b\[(\d+);(\d+);(\d+)t$/;

// Virtual key codes for special keys
export const KEY_UP = 0x1001;
export const KEY_DOWN = 0x1002;
export const KEY_RIGHT = 0x1003;
export const KEY_LEFT = 0x1004;

const ESC_MAP: Record<string, number> = {
  "\x1b[A": KEY_UP,
  "\x1b[B": KEY_DOWN,
  "\x1b[C": KEY_RIGHT,
  "\x1b[D": KEY_LEFT,
};

// ---------------------------------------------------------------------------
// CSI size response parser (exported for testing)
// ---------------------------------------------------------------------------

/** Parsed CSI size response (from CSI 14 t / CSI 18 t queries). */
export interface CsiSizeResponse {
  /** Response type: 4 = pixel size, 8 = cell count. */
  type: number;
  /** First value: pixel height (type 4) or rows (type 8). */
  a: number;
  /** Second value: pixel width (type 4) or cols (type 8). */
  b: number;
}

/**
 * Parse a CSI t size response.
 * Returns null if the input is not a valid CSI size response.
 */
export const parseCsiSizeResponse = (input: string): CsiSizeResponse | null => {
  const match = input.match(CSI_SIZE_RE);
  if (!match) return null;
  return {
    type: parseInt(match[1]),
    a: parseInt(match[2]),
    b: parseInt(match[3]),
  };
};

// ---------------------------------------------------------------------------
// SGR mouse parser (exported for testing)
// ---------------------------------------------------------------------------

/** Parsed SGR mouse event. */
export interface SgrMouseEvent {
  button: number;
  col: number;
  row: number;
  isRelease: boolean;
}

/**
 * Parse an SGR mouse escape sequence into its components.
 * Returns null if the input is not a valid SGR mouse sequence.
 */
export const parseSgrMouse = (input: string): SgrMouseEvent | null => {
  if (!input.startsWith("\x1b[<")) return null;
  const match = input.match(SGR_MOUSE_RE);
  if (!match) return null;
  return {
    button: parseInt(match[1]),
    col: parseInt(match[2]) - 1,
    row: parseInt(match[3]) - 1,
    isRelease: match[4] === "m",
  };
};

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

  // Auto-detect rendering mode: use pixel rendering on Kitty-capable terminals.
  bridge.setRenderMode("auto");

  // Enable mouse tracking (SGR mode with move events)
  process.stdout.write(MOUSE_ENABLE);

  // Query terminal pixel dimensions (fallback when ioctl fails, e.g. tmux/screen)
  process.stdout.write("\x1b[14t\x1b[18t");

  // Set viewport to actual terminal size
  const termCols = process.stdout.columns || DEFAULT_COLS;
  const termRows = process.stdout.rows || DEFAULT_ROWS;
  bridge.setViewportSize(termCols, termRows);

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
  // 3. Create EventDispatcher and wire to bridge events
  // -----------------------------------------------------------------------
  const dispatcher = new EventDispatcher(bridge, tree);
  bridge.onEvents((events) => dispatcher.handleEvents(events));

  // -----------------------------------------------------------------------
  // 4. Create React root via the reconciler
  // -----------------------------------------------------------------------
  const root = createRoot(tree);

  // -----------------------------------------------------------------------
  // 5. Render the user's element wrapped in TerminalProvider
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
  // 6. Set up stdin in raw mode for keyboard input
  // -----------------------------------------------------------------------
  const stdinDataHandler = (data: string | Buffer): void => {
    const key = typeof data === "string" ? data : data.toString("utf8");

    // Ctrl+C or 'q' triggers graceful shutdown
    if (key === CTRL_C || key === QUIT_KEY) {
      shutdown();
      return;
    }

    // Detect CSI t size responses (pixel size / cell count)
    const sizeMatch = key.match(CSI_SIZE_RE);
    if (sizeMatch) {
      const type = parseInt(sizeMatch[1]);
      const a = parseInt(sizeMatch[2]);
      const b = parseInt(sizeMatch[3]);
      if (type === 4) {
        // Pixel size response: a=height, b=width
        bridge.setTerminalPixelSize(b, a);
      } else if (type === 8) {
        // Cell count response: a=rows, b=cols
        bridge.setTerminalCellCount(b, a);
      }
      return;
    }

    // Detect SGR mouse sequences
    const mouse = parseSgrMouse(key);
    if (mouse) {
      bridge.pushMouseEvent(mouse.button, mouse.col, mouse.row, 0, 0, 0);
      dispatcher.handleMouseFromStdin(mouse.button, mouse.col, mouse.row, mouse.isRelease);
      return;
    }

    // Map escape sequences to virtual key codes; single chars pass through.
    const mapped = ESC_MAP[key];
    const keyCode = mapped ?? (key.length === 1 ? key.charCodeAt(0) : key.charCodeAt(0));
    bridge.pushKeyEvent(keyCode, 0, 0);
    dispatcher.handleStdinKeyEvent(keyCode, 0, 0);
  };

  if (process.stdin.isTTY) {
    process.stdin.setRawMode(true);
    process.stdin.resume();
    process.stdin.setEncoding("utf8");
    process.stdin.on("data", stdinDataHandler);
  }

  // -----------------------------------------------------------------------
  // 7. Start render loop
  // -----------------------------------------------------------------------
  const FRAME_MS = Math.floor(MS_PER_SECOND / fps);
  const renderLoop = setInterval(() => {
    tree.flushDirtyStyles();
    bridge.flushMutations();
    bridge.renderFrame();
  }, FRAME_MS);

  // -----------------------------------------------------------------------
  // 8. Handle terminal resize
  // -----------------------------------------------------------------------
  const resizeHandler = (): void => {
    const newCols = process.stdout.columns || DEFAULT_COLS;
    const newRows = process.stdout.rows || DEFAULT_ROWS;
    bridge.setViewportSize(newCols, newRows);
    bridge.requestRender();
  };
  process.stdout.on("resize", resizeHandler);

  // -----------------------------------------------------------------------
  // 9. Handle SIGTERM / SIGINT
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

    // Disable mouse tracking
    process.stdout.write(MOUSE_DISABLE);

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
