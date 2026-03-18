/**
 * Concrete Renderable subclasses for each JSX element type.
 */

import { type CSSStyle, type RenderableTree, Renderable } from "@kittyui/core";

/** Props shared by all KittyUI JSX elements. */
export interface KittyProps {
  children?: unknown;
  style?: CSSStyle;
}

/**
 * Flush an instance and all its pending children into the tree recursively.
 * Called when an instance is first attached to the tree (via appendChild or
 * appendChildToContainer).
 */
export const flushPendingChildren = (
  tree: RenderableTree,
  parentId: number,
  instance: BoxRenderable | TextRenderable,
): void => {
  tree.appendChild(parentId, instance);
  instance.tree = tree;
  for (const child of instance.pendingChildren) {
    flushPendingChildren(tree, instance.nodeId, child);
  }
  instance.pendingChildren = [];
};

/** A box container element (like a div). */
export class BoxRenderable extends Renderable {
  readonly type = "box";
  tree: RenderableTree | undefined;
  pendingChildren: (BoxRenderable | TextRenderable)[] = [];

  applyProps(props: KittyProps): void {
    if (props.style) {
      this.setStyle(props.style);
    }
  }
}

/** A text element that renders a string. */
export class TextRenderable extends Renderable {
  readonly type = "text";
  tree: RenderableTree | undefined;
  pendingChildren: (BoxRenderable | TextRenderable)[] = [];

  applyProps(props: KittyProps): void {
    if (props.style) {
      this.setStyle(props.style);
    }
  }
}

/** Map from JSX tag name to Renderable constructor. */
const TAG_MAP: Record<string, new () => BoxRenderable | TextRenderable> = {
  box: BoxRenderable,
  text: TextRenderable,
};

/** Create a Renderable instance from a JSX tag name. */
export const createRenderableForType = (type: string): BoxRenderable | TextRenderable => {
  const Ctor = TAG_MAP[type];
  if (!Ctor) {
    throw new Error(`Unknown KittyUI element type: "${type}"`);
  }
  return new Ctor();
};
