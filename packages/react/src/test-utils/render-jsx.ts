/**
 * render() — mount a React element into a headless KittyUI engine and
 * return a VirtualScreen with the rendered output.
 */

import type { ReactNode } from "react";
import { resetNodeIdCounter, RenderableTree, MutationEncoder } from "@kittyui/core";
import { TestBridge, VirtualScreen } from "@kittyui/core/src/test-harness/index.js";
import { createRoot, type KittyRoot } from "../reconciler.js";
import { setActiveTree } from "../host-config.js";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const DEFAULT_COLS = 40;
const DEFAULT_ROWS = 10;
const REACT_SETTLE_MS = 50;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface RenderResult {
  /** The virtual screen containing the rendered output. */
  screen: VirtualScreen;
  /** Re-render with a new element and return an updated screen. */
  rerender: (element: ReactNode) => Promise<VirtualScreen>;
  /** Tear down the engine. */
  cleanup: () => void;
  /** Get the computed layout for a node. */
  getLayout: (nodeId: number) => { x: number; y: number; width: number; height: number };
  /** Get all computed layouts. */
  getAllLayouts: () => Map<number, { x: number; y: number; width: number; height: number }>;
}

// ---------------------------------------------------------------------------
// render()
// ---------------------------------------------------------------------------

export const render = async (
  element: ReactNode,
  options?: { cols?: number; rows?: number },
): Promise<RenderResult> => {
  const cols = options?.cols ?? DEFAULT_COLS;
  const rows = options?.rows ?? DEFAULT_ROWS;

  // 1. Reset IDs for deterministic tests.
  resetNodeIdCounter();

  // 2. Create test bridge (headless — no terminal).
  const bridge = new TestBridge();
  bridge.initTestMode(cols, rows);

  // 3. Wire up encoder + tree.
  const encoder = bridge.getEncoder();
  encoder.setViewportSize(cols, rows);
  const tree = new RenderableTree(encoder);

  // 4. Create React root (this also sets activeTree and root style).
  const root: KittyRoot = createRoot(tree);

  // Override root dimensions to match test viewport.
  const rootRenderable = tree.get(tree.root!);
  if (rootRenderable) {
    rootRenderable.setNodeStyle({ width: { type: "cells", value: cols }, height: { type: "cells", value: rows } });
  }

  // 5. Render element.
  root.render(element);

  // 6. Wait for React to settle.
  await new Promise((resolve) => setTimeout(resolve, REACT_SETTLE_MS));

  // 7. Flush and render.
  const flush = (): VirtualScreen => {
    tree.flushDirtyStyles();
    bridge.flushMutations();
    bridge.renderFrame();

    const output = bridge.getRenderedOutput();
    const screen = new VirtualScreen(cols, rows);
    if (output.length > 0) {
      screen.apply(output);
    }
    return screen;
  };

  const screen = flush();

  // 8. Build result.
  const rerender = async (newElement: ReactNode): Promise<VirtualScreen> => {
    root.render(newElement);
    await new Promise((resolve) => setTimeout(resolve, REACT_SETTLE_MS));
    return flush();
  };

  const cleanup = (): void => {
    root.unmount();
    setActiveTree(undefined);
    bridge.shutdownTestMode();
  };

  const getLayout = (nodeId: number) => bridge.getLayout(nodeId);
  const getAllLayouts = () => bridge.getAllLayouts();

  return { cleanup, getAllLayouts, getLayout, rerender, screen };
};
