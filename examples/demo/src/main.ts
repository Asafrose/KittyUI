/**
 * KittyUI Demo — entry point.
 *
 * Creates the rendering pipeline using the React reconciler:
 *   Bridge -> MutationEncoder -> RenderableTree -> createRoot -> JSX
 */

import { createElement } from "react";
import { Bridge, MutationEncoder, RenderableTree } from "@kittyui/core";
import { createRoot } from "@kittyui/react";
import { App } from "./app.js";

// ---------------------------------------------------------------------------
// Bootstrap
// ---------------------------------------------------------------------------

const bridge = new Bridge();

let nativeReady = false;

if (bridge.nativeAvailable) {
  const info = bridge.init();
  console.log(
    `KittyUI native engine v${info.versionMajor}.${info.versionMinor}.${info.versionPatch}` +
      ` (batched FFI: ${info.batchedFfi})`,
  );
  nativeReady = true;
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
// Render loop — wait for React to flush, then start the native render loop
// ---------------------------------------------------------------------------

// React concurrent mode schedules work asynchronously. Use setTimeout to
// give the reconciler a tick to commit the initial tree before we flush
// mutations to Rust and start the render loop.
setTimeout(() => {
  console.log(`Tree built: ${tree.size} nodes`);

  if (nativeReady) {
    tree.flushDirtyStyles();
    bridge.flushMutations();
    bridge.startRenderLoop();

    // Keep the process alive — the render loop runs on a background Rust thread.
    const keepAlive = setInterval(() => {}, 1 << 30);

    process.on("SIGINT", () => {
      clearInterval(keepAlive);
      bridge.stopRenderLoop();
      root.unmount();
      bridge.shutdown();
      process.exit(0);
    });

    console.log("Demo running — press Ctrl+C to exit.");
  } else {
    console.log("React tree mounted successfully (no rendering without native engine).");
  }
}, 0);
