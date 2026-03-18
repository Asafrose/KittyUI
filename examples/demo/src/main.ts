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

// Mount the React component tree
root.render(createElement(App));

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

  // Render loop
  const renderLoop = setInterval(() => {
    renderer.renderFrame();
  }, FRAME_MS);

  process.on("SIGINT", () => {
    clearInterval(renderLoop);
    renderer.cleanup();
    root.unmount();
    process.exit(0);
  });
}, 0);
