/**
 * RenderableTree — manages parent-child relationships between Renderable
 * components and syncs mutations to the Rust layout tree via the Bridge.
 */

import type { MutationEncoder } from "./mutation-encoder.js";
import type { Renderable } from "./renderable.js";
import type { Color, ComputedLayout } from "./types.js";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const NOT_FOUND = -1;
const SPLICE_DELETE_COUNT = 1;
const HEX_RADIX = 16;
const BYTE_PAD_LEN = 2;

/** Convert a Color to a hex string like `#rrggbb`. */
const colorToHex = (color: Color): string | undefined => {
  if (color.type === "rgb") {
    const r = color.r.toString(HEX_RADIX).padStart(BYTE_PAD_LEN, "0");
    const g = color.g.toString(HEX_RADIX).padStart(BYTE_PAD_LEN, "0");
    const b = color.b.toString(HEX_RADIX).padStart(BYTE_PAD_LEN, "0");
    return `#${r}${g}${b}`;
  }
  return undefined;
};

/** Merge textStyle visual properties into a style record so they reach Rust. */
const mergeVisualStyle = (style: Record<string, unknown>, renderable: Renderable): Record<string, unknown> => {
  const ts = renderable.textStyle;
  const result = { ...style };
  if (ts.bg) {
    const hex = colorToHex(ts.bg);
    if (hex) result.backgroundColor = hex;
  }
  if (ts.fg) {
    const hex = colorToHex(ts.fg);
    if (hex) result.color = hex;
  }
  if (ts.bold) {
    result.bold = true;
  }
  if (ts.italic) {
    result.italic = true;
  }
  if (ts.textOverflow) {
    result.textOverflow = ts.textOverflow;
  }
  if (ts.underline) {
    result.underline = true;
  }
  if (ts.strikethrough) {
    result.strikethrough = true;
  }
  if (ts.dim) {
    result.dim = true;
  }
  return result;
};

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
   * Register a renderable as an orphan (no parent yet).
   * Called by the reconciler's createInstance before the node is appended.
   */
  addOrphan(renderable: Renderable): void {
    if (this.nodes.has(renderable.nodeId)) return;
    this.createNode(renderable, undefined);
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

    const existing = this.nodes.get(child.nodeId);
    if (existing) {
      // Already registered as orphan — just update parent.
      existing.parent = parentId;
    } else {
      this.createNode(child, parentId);
    }
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

    const existing = this.nodes.get(child.nodeId);
    if (existing) {
      existing.parent = parentId;
    } else {
      this.createNode(child, parentId);
    }
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
        const style = mergeVisualStyle(
          node.renderable.nodeStyle as Record<string, unknown>,
          node.renderable,
        );
        this.encoder.setStyle(node.renderable.nodeId, style);
        if (node.renderable.text !== undefined) {
          this.encoder.setText(node.renderable.nodeId, node.renderable.text);
        }
        if (node.renderable.colorSpans.length > 0) {
          this.encoder.setTextSpans(node.renderable.nodeId, node.renderable.colorSpans);
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

    // Encode creation — merge visual style colors into the style blob.
    const style = mergeVisualStyle(
      renderable.nodeStyle as Record<string, unknown>,
      renderable,
    );
    this.encoder.createNode(nodeId, style);

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
