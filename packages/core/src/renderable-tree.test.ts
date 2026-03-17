import { describe, expect, it, beforeEach } from "bun:test";
import { Renderable, resetNodeIdCounter } from "./renderable.js";
import { RenderableTree } from "./renderable-tree.js";
import { MutationEncoder } from "./mutation-encoder.js";
import type { ComputedLayout } from "./types.js";

class TestRenderable extends Renderable {
  mountCalled = false;
  unmountCalled = false;
  lastLayout: ComputedLayout | undefined;

  onMount(): void {
    this.mountCalled = true;
  }

  onUnmount(): void {
    this.unmountCalled = true;
  }

  onLayout(layout: ComputedLayout): void {
    this.lastLayout = layout;
  }
}

describe("RenderableTree", () => {
  let encoder: MutationEncoder;
  let tree: RenderableTree;

  beforeEach(() => {
    resetNodeIdCounter();
    encoder = new MutationEncoder();
    tree = new RenderableTree(encoder);
  });

  // -----------------------------------------------------------------------
  // Basic operations
  // -----------------------------------------------------------------------

  it("starts empty", () => {
    expect(tree.size).toBe(0);
    expect(tree.root).toBeUndefined();
  });

  it("setRoot adds a root node", () => {
    const root = new TestRenderable();
    tree.setRoot(root);
    expect(tree.root).toBe(root.nodeId);
    expect(tree.size).toBe(1);
    expect(tree.get(root.nodeId)).toBe(root);
  });

  it("setRoot calls onMount", () => {
    const root = new TestRenderable();
    tree.setRoot(root);
    expect(root.mountCalled).toBe(true);
  });

  it("setRoot replaces existing root", () => {
    const a = new TestRenderable();
    const b = new TestRenderable();
    tree.setRoot(a);
    tree.setRoot(b);
    expect(tree.root).toBe(b.nodeId);
    expect(tree.size).toBe(1);
    expect(a.unmountCalled).toBe(true);
  });

  it("setRoot encodes createNode mutation", () => {
    const root = new TestRenderable();
    tree.setRoot(root);
    expect(encoder.byteLength).toBeGreaterThan(0);
  });

  // -----------------------------------------------------------------------
  // appendChild
  // -----------------------------------------------------------------------

  it("appends a child", () => {
    const root = new TestRenderable();
    const child = new TestRenderable();
    tree.setRoot(root);
    tree.appendChild(root.nodeId, child);
    expect(tree.size).toBe(2);
    expect(tree.children(root.nodeId)).toEqual([child]);
    expect(tree.parent(child.nodeId)).toBe(root);
  });

  it("appends multiple children in order", () => {
    const root = new TestRenderable();
    const a = new TestRenderable();
    const b = new TestRenderable();
    tree.setRoot(root);
    tree.appendChild(root.nodeId, a);
    tree.appendChild(root.nodeId, b);
    expect(tree.children(root.nodeId).map((r) => r.nodeId)).toEqual([
      a.nodeId,
      b.nodeId,
    ]);
  });

  it("throws when appending to non-existent parent", () => {
    const child = new TestRenderable();
    expect(() => tree.appendChild(999, child)).toThrow("Parent node 999 not found");
  });

  // -----------------------------------------------------------------------
  // insertBefore
  // -----------------------------------------------------------------------

  it("inserts before a sibling", () => {
    const root = new TestRenderable();
    const a = new TestRenderable();
    const b = new TestRenderable();
    const c = new TestRenderable();
    tree.setRoot(root);
    tree.appendChild(root.nodeId, a);
    tree.appendChild(root.nodeId, b);
    tree.insertBefore(root.nodeId, c, b.nodeId);
    expect(tree.children(root.nodeId).map((r) => r.nodeId)).toEqual([
      a.nodeId,
      c.nodeId,
      b.nodeId,
    ]);
  });

  it("throws when inserting before non-existent sibling", () => {
    const root = new TestRenderable();
    const child = new TestRenderable();
    tree.setRoot(root);
    expect(() => tree.insertBefore(root.nodeId, child, 999)).toThrow(
      "Reference node 999 is not a child of",
    );
  });

  // -----------------------------------------------------------------------
  // remove
  // -----------------------------------------------------------------------

  it("removes a leaf node", () => {
    const root = new TestRenderable();
    const child = new TestRenderable();
    tree.setRoot(root);
    tree.appendChild(root.nodeId, child);
    tree.remove(child.nodeId);
    expect(tree.size).toBe(1);
    expect(tree.children(root.nodeId)).toEqual([]);
    expect(child.unmountCalled).toBe(true);
  });

  it("removes a subtree recursively", () => {
    const root = new TestRenderable();
    const parent = new TestRenderable();
    const child = new TestRenderable();
    tree.setRoot(root);
    tree.appendChild(root.nodeId, parent);
    tree.appendChild(parent.nodeId, child);
    tree.remove(parent.nodeId);
    expect(tree.size).toBe(1);
    expect(child.unmountCalled).toBe(true);
    expect(parent.unmountCalled).toBe(true);
  });

  it("removing root clears root", () => {
    const root = new TestRenderable();
    tree.setRoot(root);
    tree.remove(root.nodeId);
    expect(tree.root).toBeUndefined();
    expect(tree.size).toBe(0);
  });

  it("removing non-existent node is a no-op", () => {
    tree.remove(999);
    expect(tree.size).toBe(0);
  });

  // -----------------------------------------------------------------------
  // flushDirtyStyles
  // -----------------------------------------------------------------------

  it("flushes dirty styles to encoder", () => {
    const root = new TestRenderable();
    tree.setRoot(root);
    encoder.reset();

    root.setStyle({ width: 40 });
    tree.flushDirtyStyles();
    expect(encoder.byteLength).toBeGreaterThan(0);
    expect(root.dirty).toBe(false);
  });

  it("does not re-flush clean nodes", () => {
    const root = new TestRenderable();
    tree.setRoot(root);
    encoder.reset();

    tree.flushDirtyStyles();
    const lenAfterClean = encoder.byteLength;
    tree.flushDirtyStyles();
    expect(encoder.byteLength).toBe(lenAfterClean);
  });

  // -----------------------------------------------------------------------
  // applyLayouts
  // -----------------------------------------------------------------------

  it("updates layouts on renderables", () => {
    const root = new TestRenderable();
    const child = new TestRenderable();
    tree.setRoot(root);
    tree.appendChild(root.nodeId, child);

    const layouts = new Map<number, ComputedLayout>();
    layouts.set(root.nodeId, { x: 0, y: 0, width: 80, height: 24 });
    layouts.set(child.nodeId, { x: 5, y: 3, width: 20, height: 10 });

    tree.applyLayouts(layouts);
    expect(root.layout).toEqual({ x: 0, y: 0, width: 80, height: 24 });
    expect(child.layout).toEqual({ x: 5, y: 3, width: 20, height: 10 });
    expect((child as TestRenderable).lastLayout).toEqual({
      x: 5,
      y: 3,
      width: 20,
      height: 10,
    });
  });

  // -----------------------------------------------------------------------
  // Edge cases
  // -----------------------------------------------------------------------

  it("children() returns empty for non-existent node", () => {
    expect(tree.children(999)).toEqual([]);
  });

  it("parent() returns undefined for root", () => {
    const root = new TestRenderable();
    tree.setRoot(root);
    expect(tree.parent(root.nodeId)).toBeUndefined();
  });

  it("get() returns undefined for non-existent node", () => {
    expect(tree.get(999)).toBeUndefined();
  });
});
