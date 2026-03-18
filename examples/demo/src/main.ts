/**
 * KittyUI Demo — entry point.
 *
 * Creates the rendering pipeline manually using the lower-level APIs:
 *   Bridge -> MutationEncoder -> RenderableTree -> React reconciler
 */

import { createElement } from "react";
import { Bridge, MutationEncoder, RenderableTree } from "@kittyui/core";
import { createRoot } from "@kittyui/react";
import { App } from "./app.js";

// ---------------------------------------------------------------------------
// Bootstrap
// ---------------------------------------------------------------------------

const bridge = new Bridge();

// Initialize the native Rust engine (requires a pre-built native library).
// If the native library is not available, we still set up the tree so the
// React reconciler can run — useful for type-checking and testing the
// component tree without a terminal attached.
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
// Render loop (only when native is available)
// ---------------------------------------------------------------------------

if (nativeReady) {
  // Flush the initial mutations to Rust
  tree.flushDirtyStyles();
  bridge.flushMutations();

  // Start the FPS-capped render loop on the Rust side
  bridge.startRenderLoop();

  // Handle graceful shutdown
  process.on("SIGINT", () => {
    bridge.stopRenderLoop();
    root.unmount();
    bridge.shutdown();
    process.exit(0);
  });

  console.log("Demo running — press Ctrl+C to exit.");
} else {
  console.log("React tree mounted successfully (no rendering without native engine).");
  console.log(`Tree size: ${tree.size} nodes`);
}
