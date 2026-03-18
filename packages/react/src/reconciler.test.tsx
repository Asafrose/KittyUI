import { BoxRenderable, TextRenderable, createRoot } from "./index.js";
import { type MutationEncoder, RenderableTree, resetNodeIdCounter } from "@kittyui/core";
import { describe, expect, test } from "bun:test";
import React from "react";

/**
 * Stub MutationEncoder that records calls for assertion.
 */
const FLUSH_DELAY_MS = 50;
const FIRST_CHILD = 0;
const ONE_CHILD = 1;
const ZERO_CHILDREN = 0;

const createStubEncoder = (): MutationEncoder =>
  ({
    appendChild() {},
    createNode() {},
    insertBefore() {},
    removeNode() {},
    setStyle() {},
    setText() {},
  }) as unknown as MutationEncoder;

const createTree = (): RenderableTree => {
  resetNodeIdCounter();
  return new RenderableTree(createStubEncoder());
};

const flush = (): Promise<void> =>
  new Promise((resolve) => {
    setTimeout(resolve, FLUSH_DELAY_MS);
  });

describe("createRoot", () => {
  test("creates a root with a BoxRenderable as the tree root", () => {
    const tree = createTree();
    const root = createRoot(tree);
    expect(root).toBeDefined();
    expect(root.render).toBeFunction();
    expect(root.unmount).toBeFunction();
    expect(tree.root).toBeDefined();
  });

  test("render() mounts a simple box element into the tree", async () => {
    const tree = createTree();
    const root = createRoot(tree);

    root.render(React.createElement("box", { style: { width: 10 } }));

    await flush();

    const rootId = tree.root!;
    const children = tree.children(rootId);
    expect(children.length).toBe(ONE_CHILD);
    expect(children[FIRST_CHILD]).toBeInstanceOf(BoxRenderable);
  });

  test("render() mounts a text element with content", async () => {
    const tree = createTree();
    const root = createRoot(tree);

    root.render(React.createElement("text", undefined, "hello"));

    await flush();

    const rootId = tree.root!;
    const children = tree.children(rootId);
    // Text element + text instance child
    expect(children.length).toBeGreaterThanOrEqual(ONE_CHILD);
  });

  test("render() mounts nested elements", async () => {
    const tree = createTree();
    const root = createRoot(tree);

    root.render(
      React.createElement(
        "box",
        { style: { width: 80 } },
        React.createElement("text", undefined, "child 1"),
        React.createElement("text", undefined, "child 2"),
      ),
    );

    await flush();

    const rootId = tree.root!;
    const children = tree.children(rootId);
    expect(children.length).toBe(ONE_CHILD);
  });

  test("unmount() clears the tree", async () => {
    const tree = createTree();
    const root = createRoot(tree);

    root.render(React.createElement("box"));

    await flush();

    const rootId = tree.root!;
    expect(tree.children(rootId).length).toBe(ONE_CHILD);

    root.unmount();

    await flush();

    expect(tree.children(rootId).length).toBe(ZERO_CHILDREN);
  });

  test("re-render updates the tree", async () => {
    const tree = createTree();
    const root = createRoot(tree);

    root.render(React.createElement("box", { style: { width: 10 } }));

    await flush();

    root.render(React.createElement("box", { style: { width: 20 } }));

    await flush();

    const rootId = tree.root!;
    const children = tree.children(rootId);
    expect(children.length).toBe(ONE_CHILD);
  });
});

describe("renderables", () => {
  test("BoxRenderable applies style props", () => {
    const box = new BoxRenderable();
    box.applyProps({ style: { width: 42 } });
    expect(box.nodeStyle).toBeDefined();
    expect(box.type).toBe("box");
  });

  test("TextRenderable applies style and text", () => {
    const text = new TextRenderable();
    text.applyProps({ style: { width: 10 } });
    text.setText("hello");
    expect(text.text).toBe("hello");
    expect(text.type).toBe("text");
  });
});
