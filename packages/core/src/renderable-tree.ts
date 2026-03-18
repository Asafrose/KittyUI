/**
 * RenderableTree — manages parent-child relationships between Renderable
 * components and syncs mutations to the Rust layout tree via the Bridge.
 */

import type { MutationEncoder } from "./mutation-encoder.js";
import type { Renderable } from "./renderable.js";
import type { ComputedLayout } from "./types.js";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const NOT_FOUND = -1;
const SPLICE_DELETE_COUNT = 1;

// ---------------------------------------------------------------------------
// Tree node metadata
// ---------------------------------------------------------------------------

interface TreeNode {
  children: number[];
  parent: number | undefined;
  renderable: Renderable;
}

// ---------------------------------------------------------------------------
// RenderableTree
// ---------------------------------------------------------------------------

export class RenderableTree {
  private nodes = new Map<number, TreeNode>();
  private rootId: number | undefined;
  private encoder: MutationEncoder;

  constructor(encoder: MutationEncoder) {
    this.encoder = encoder;
  }

  /** Get the root node ID. */
  get root(): number | undefined {
    return this.rootId;
  }

  /** Get a renderable by node ID. */
  get(nodeId: number): Renderable | undefined {
    return this.nodes.get(nodeId)?.renderable;
  }

  /** Get the children of a node. */
  children(nodeId: number): Renderable[] {
    const node = this.nodes.get(nodeId);
    if (!node) {
      return [];
    }
    return node.children
      .map((id) => this.nodes.get(id)?.renderable)
      .filter((item): item is Renderable => item !== undefined);
  }

  /** Get the parent of a node. */
  parent(nodeId: number): Renderable | undefined {
    const node = this.nodes.get(nodeId);
    if (!node || node.parent === undefined) {
      return undefined;
    }
    return this.nodes.get(node.parent)?.renderable;
  }

  /** Total number of nodes in the tree. */
  get size(): number {
    return this.nodes.size;
  }

  // -----------------------------------------------------------------------
  // Tree mutations
  // -----------------------------------------------------------------------

  /**
   * Set the root renderable. Creates the node in the Rust layout tree.
   * If a root already exists, it is replaced.
   */
  setRoot(renderable: Renderable): void {
    if (this.rootId !== undefined) {
      this.remove(this.rootId);
    }
    this.createNode(renderable, undefined);
    this.rootId = renderable.nodeId;
  }

  /**
   * Append a child renderable to a parent.
   * Creates the child node and encodes the appendChild mutation.
   */
  appendChild(parentId: number, child: Renderable): void {
    const parentNode = this.nodes.get(parentId);
    if (!parentNode) {
      throw new Error(`Parent node ${parentId} not found`);
    }

    this.createNode(child, parentId);
    parentNode.children.push(child.nodeId);

    this.encoder.appendChild(parentId, child.nodeId);
  }

  /**
   * Insert a child before a reference sibling.
   */
  insertBefore(parentId: number, child: Renderable, beforeId: number): void {
    const parentNode = this.nodes.get(parentId);
    if (!parentNode) {
      throw new Error(`Parent node ${parentId} not found`);
    }

    const idx = parentNode.children.indexOf(beforeId);
    if (idx === NOT_FOUND) {
      throw new Error(`Reference node ${beforeId} is not a child of ${parentId}`);
    }

    this.createNode(child, parentId);
    parentNode.children.splice(idx, 0, child.nodeId);

    this.encoder.insertBefore(parentId, child.nodeId, beforeId);
  }

  /**
   * Remove a node and all its descendants from the tree.
   */
  remove(nodeId: number): void {
    const node = this.nodes.get(nodeId);
    if (!node) {
      return;
    }

    // Remove children recursively (copy array since we mutate)
    for (const childId of Array.from(node.children)) {
      this.remove(childId);
    }

    // Remove from parent's children list
    if (node.parent !== undefined) {
      const parentNode = this.nodes.get(node.parent);
      if (parentNode) {
        const idx = parentNode.children.indexOf(nodeId);
        if (idx !== NOT_FOUND) {
          parentNode.children.splice(idx, SPLICE_DELETE_COUNT);
        }
      }
    }

    // Encode removal
    this.encoder.removeNode(nodeId);

    // Lifecycle
    node.renderable.onUnmount();

    this.nodes.delete(nodeId);

    if (this.rootId === nodeId) {
      this.rootId = undefined;
    }
  }

  // -----------------------------------------------------------------------
  // Sync
  // -----------------------------------------------------------------------

  /**
   * Flush all dirty node styles to the encoder.
   * Call this before flushing mutations via Bridge.
   */
  flushDirtyStyles(): void {
    for (const [, node] of this.nodes) {
      if (node.renderable.dirty) {
        this.encoder.setStyle(node.renderable.nodeId, node.renderable.nodeStyle as Record<string, unknown>);
        if (node.renderable.text !== undefined) {
          this.encoder.setText(node.renderable.nodeId, node.renderable.text);
        }
        node.renderable.clearDirty();
      }
    }
  }

  /**
   * Update computed layouts on all renderables from a layout map.
   * Typically called after Bridge.getAllLayouts().
   */
  applyLayouts(layouts: Map<number, ComputedLayout>): void {
    for (const [nodeId, layout] of layouts) {
      const node = this.nodes.get(nodeId);
      if (node) {
        node.renderable.updateLayout(layout);
        node.renderable.onLayout(layout);
      }
    }
  }

  // -----------------------------------------------------------------------
  // Internal
  // -----------------------------------------------------------------------

  private createNode(renderable: Renderable, parentId: number | undefined): void {
    const nodeId = renderable.nodeId;

    this.nodes.set(nodeId, {
      children: [],
      parent: parentId,
      renderable,
    });

    // Encode creation
    this.encoder.createNode(nodeId, renderable.nodeStyle as Record<string, unknown>);

    // Set text if present
    if (renderable.text !== undefined) {
      this.encoder.setText(nodeId, renderable.text);
    }

    // Mark as clean after encoding
    renderable.clearDirty();

    // Lifecycle
    renderable.onMount();
  }
}
