/**
 * KittyUI Demo — entry point.
 *
 * Creates the rendering pipeline using the React reconciler:
 *   Bridge -> MutationEncoder -> RenderableTree -> createRoot -> JSX
 *
 * Then renders to the terminal using a simple TS-side ANSI renderer
 * that reads computed layouts from the Rust engine.
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
  const info = bridge.init();
  nativeReady = true;
  void info; // Used below after React mounts
} else {
  console.log("Native library not found — running in tree-only mode.");
  console.log("Build the native library with: bun run build:native");
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
  if (!nativeReady) {
    console.log(`React tree mounted: ${tree.size} nodes (no rendering without native engine).`);
    return;
  }

  const renderer = new DemoRenderer({ bridge, tree });
  renderer.setup();

  // Initial render
  renderer.renderFrame();

  // Render loop on the TS side
  const renderLoop = setInterval(() => {
    renderer.renderFrame();
  }, FRAME_MS);

  process.on("SIGINT", () => {
    clearInterval(renderLoop);
    renderer.cleanup();
    root.unmount();
    bridge.shutdown();
    process.exit(0);
  });
}, 0);
