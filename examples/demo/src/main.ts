/**
 * KittyUI Demo — entry point.
 *
 * Creates the rendering pipeline using the React reconciler:
 *   Bridge -> MutationEncoder -> RenderableTree -> createRoot -> JSX
 *
 * Then renders to the terminal using a TS-side ANSI renderer.
 * Works with or without the native Rust engine.
 */

import { createElement } from "react";
import { Bridge, MutationEncoder, RenderableTree } from "@kittyui/core";
import { createRoot } from "@kittyui/react";
import { App } from "./app.js";
import { DemoRenderer } from "./renderer.js";

// ---------------------------------------------------------------------------
// Bootstrap
// ---------------------------------------------------------------------------

const bridge = new Bridge();

let nativeReady = false;

if (bridge.nativeAvailable) {
  bridge.init();
  nativeReady = true;
} else {
  // Works fine without native — uses pure-TS layout engine
}

// ---------------------------------------------------------------------------
// Create the rendering pipeline
// ---------------------------------------------------------------------------

const encoder = nativeReady ? bridge.getEncoder() : new MutationEncoder();
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
  const renderer = new DemoRenderer({
    bridge: nativeReady ? bridge : undefined,
    tree,
  });
  renderer.setup();

  // Initial render
  renderer.renderFrame();

  // Render loop
  const renderLoop = setInterval(() => {
    renderer.renderFrame();
  }, FRAME_MS);

  process.on("SIGINT", () => {
    clearInterval(renderLoop);
    renderer.cleanup();
    root.unmount();
    if (nativeReady) bridge.shutdown();
    process.exit(0);
  });
}, 0);
