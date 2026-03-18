/**
 * KittyUI React reconciler — createRoot / root.render() entry point.
 */

import { type Container, hostConfig } from "./host-config.js";
import { BoxRenderable } from "./renderables.js";
import type { ReactNode } from "react";
import ReactReconciler from "react-reconciler";
import type { RenderableTree } from "@kittyui/core";

// ---------------------------------------------------------------------------
// Reconciler instance
// ---------------------------------------------------------------------------

// eslint-disable-next-line new-cap
const reconciler = ReactReconciler(hostConfig as Parameters<typeof ReactReconciler>[0]); // eslint-disable-line no-magic-numbers

// ---------------------------------------------------------------------------
// ConcurrentRoot tag (react-reconciler uses 1 for ConcurrentRoot)
// ---------------------------------------------------------------------------

const CONCURRENT_ROOT = 1;

// ---------------------------------------------------------------------------
// Error handlers
// ---------------------------------------------------------------------------

const noop = (): void => {};

// ---------------------------------------------------------------------------
// Root
// ---------------------------------------------------------------------------

export interface KittyRoot {
  render(element: ReactNode): void;
  unmount(): void;
}

/**
 * Create a KittyUI root bound to a RenderableTree.
 *
 * Usage:
 * ```ts
 * const root = createRoot(tree);
 * root.render(<App />);
 * ```
 */
export const createRoot = (tree: RenderableTree): KittyRoot => {
  const rootRenderable = new BoxRenderable();
  tree.setRoot(rootRenderable);

  const container: Container = { root: rootRenderable, tree };

  const fiberRoot = reconciler.createContainer(
    container,
    CONCURRENT_ROOT,
    // eslint-disable-next-line unicorn/no-null -- Required by react-reconciler API
    null,
    false,
    // eslint-disable-next-line unicorn/no-null -- Required by react-reconciler API
    null,
    "",
    noop,
    noop,
    noop,
    noop,
  );

  return {
    render(element: ReactNode): void {
      reconciler.updateContainer(element, fiberRoot, undefined, undefined);
    },
    unmount(): void {
      reconciler.updateContainer(undefined, fiberRoot, undefined, undefined);
    },
  };
};
