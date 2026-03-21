/**
 * KittyUI Demo — entry point.
 *
 * Creates the rendering pipeline using the React reconciler:
 *   MutationEncoder -> RenderableTree -> createRoot -> JSX
 *
 * Then renders to the terminal using a TS-side ANSI renderer
 * with a pure-TS flexbox layout engine.
 */

import { createElement } from "react";
import { MutationEncoder, RenderableTree } from "@kittyui/core";
import { createRoot } from "@kittyui/react";
import { App } from "./app.js";
import type { AppProps } from "./app.js";
import { DemoRenderer } from "./renderer.js";

// ---------------------------------------------------------------------------
// CLI flags
// ---------------------------------------------------------------------------

const debug = process.argv.includes("--debug");

// ---------------------------------------------------------------------------
// Create the rendering pipeline
// ---------------------------------------------------------------------------

const encoder = new MutationEncoder();
const tree = new RenderableTree(encoder);
const root = createRoot(tree);

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

const SIDEBAR_COUNT = 4;
let activeIndex = 0;

const renderApp = () => {
  root.render(createElement(App, { activeIndex } satisfies AppProps));
};

// Initial mount
renderApp();

// ---------------------------------------------------------------------------
// Start rendering after React commits the initial tree
// ---------------------------------------------------------------------------

const FPS = 30;
const FRAME_MS = Math.floor(1000 / FPS);

setTimeout(() => {
  const renderer = new DemoRenderer({ tree, debug });
  renderer.setup();

  // Initial render
  renderer.renderFrame();

  if (debug) {
    // In debug mode, render once and exit
    renderer.cleanup();
    root.unmount();
    process.exit(0);
  }

  // ---------------------------------------------------------------------------
  // Keyboard input — raw mode stdin
  // ---------------------------------------------------------------------------

  if (process.stdin.isTTY) {
    process.stdin.setRawMode(true);
    process.stdin.resume();
    process.stdin.setEncoding("utf8");

    process.stdin.on("data", (key: string) => {
      // Ctrl+C or 'q' to quit
      if (key === "\x03" || key === "q") {
        cleanup();
        return;
      }

      // Arrow up
      if (key === "\x1b[A" || key === "k") {
        activeIndex = (activeIndex - 1 + SIDEBAR_COUNT) % SIDEBAR_COUNT;
        renderApp();
        // Force immediate re-render so it feels responsive
        setTimeout(() => renderer.renderFrame(), 0);
        return;
      }

      // Arrow down
      if (key === "\x1b[B" || key === "j") {
        activeIndex = (activeIndex + 1) % SIDEBAR_COUNT;
        renderApp();
        setTimeout(() => renderer.renderFrame(), 0);
        return;
      }
    });
  }

  // ---------------------------------------------------------------------------
  // Render loop
  // ---------------------------------------------------------------------------

  const renderLoop = setInterval(() => {
    renderer.renderFrame();
  }, FRAME_MS);

  const cleanup = () => {
    clearInterval(renderLoop);
    if (process.stdin.isTTY) {
      process.stdin.setRawMode(false);
    }
    renderer.cleanup();
    root.unmount();
    process.exit(0);
  };

  process.on("SIGINT", cleanup);
  process.on("SIGTERM", cleanup);
}, 0);
