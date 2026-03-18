/**
 * Simple pure-TS flexbox layout engine for the demo.
 *
 * Walks the RenderableTree and computes ComputedLayout for each node
 * based on NodeStyle properties. This is a minimal implementation that
 * handles the subset of flexbox needed for the dashboard demo:
 *   - flexDirection: row / column
 *   - width / height (cells and percent)
 *   - flexGrow
 *   - padding (uniform)
 *   - gap
 */

import type { ComputedLayout, Dim, NodeStyle, RenderableTree } from "@kittyui/core";

/** Resolve a Dim to a cell value given the parent's available size. */
const resolveDim = (dim: Dim | undefined, available: number): number | undefined => {
  if (!dim) return undefined;
  if (dim.type === "cells") return dim.value;
  if (dim.type === "percent") return Math.floor((dim.value / 100) * available);
  return undefined; // auto
};

/** Extract uniform padding (top value) from a NodeStyle. */
const getPadding = (style: NodeStyle): number => {
  if (!style.padding) return 0;
  // padding is [top, right, bottom, left] of Dim
  const top = style.padding[0];
  return top.type === "cells" ? top.value : 0;
};

const getPaddingH = (style: NodeStyle): number => {
  if (!style.padding) return 0;
  const right = style.padding[1];
  const left = style.padding[3];
  const r = right.type === "cells" ? right.value : 0;
  const l = left.type === "cells" ? left.value : 0;
  return r + l;
};

const getPaddingV = (style: NodeStyle): number => {
  if (!style.padding) return 0;
  const top = style.padding[0];
  const bottom = style.padding[2];
  const t = top.type === "cells" ? top.value : 0;
  const b = bottom.type === "cells" ? bottom.value : 0;
  return t + b;
};

const getPaddingTop = (style: NodeStyle): number => {
  if (!style.padding) return 0;
  const top = style.padding[0];
  return top.type === "cells" ? top.value : 0;
};

const getPaddingLeft = (style: NodeStyle): number => {
  if (!style.padding) return 0;
  const left = style.padding[3];
  return left.type === "cells" ? left.value : 0;
};

/** Get the gap between children. */
const getGap = (style: NodeStyle, isRow: boolean): number => {
  if (!style.gap) return 0;
  // gap is [column, row]
  const dim = isRow ? style.gap[0] : style.gap[1];
  return dim.type === "cells" ? dim.value : 0;
};

/** Get flex direction — defaults to column. */
const getDirection = (style: NodeStyle): "row" | "column" => {
  if (style.display?.type === "flex" && style.display.flex?.direction) {
    return style.display.flex.direction as "row" | "column";
  }
  return "column";
};

/** Get flex grow for a node. */
const getFlexGrow = (style: NodeStyle): number => {
  if (style.display?.type === "flex" && style.display.flex?.grow !== undefined) {
    return style.display.flex.grow;
  }
  return 0;
};

/** Get margin-top. */
const getMarginTop = (style: NodeStyle): number => {
  if (!style.margin) return 0;
  const top = style.margin[0];
  return top.type === "cells" ? top.value : 0;
};

/**
 * Compute layouts for the entire tree, starting from the root.
 * Returns a Map of nodeId -> ComputedLayout.
 */
export const computeLayouts = (
  tree: RenderableTree,
  containerWidth: number,
  containerHeight: number,
): Map<number, ComputedLayout> => {
  const layouts = new Map<number, ComputedLayout>();
  const rootId = tree.root;
  if (rootId === undefined) return layouts;

  layoutNode(tree, rootId, 0, 0, containerWidth, containerHeight, layouts);
  return layouts;
};

const layoutNode = (
  tree: RenderableTree,
  nodeId: number,
  x: number,
  y: number,
  availW: number,
  availH: number,
  layouts: Map<number, ComputedLayout>,
): void => {
  const renderable = tree.get(nodeId);
  if (!renderable) return;

  const style = renderable.nodeStyle;

  // Resolve own dimensions
  const w = resolveDim(style.width, availW) ?? availW;
  const h = resolveDim(style.height, availH) ?? availH;

  layouts.set(nodeId, { x, y, width: w, height: h });

  // Layout children
  const children = tree.children(nodeId);
  if (children.length === 0) return;

  const direction = getDirection(style);
  const isRow = direction === "row";
  const padH = getPaddingH(style);
  const padV = getPaddingV(style);
  const padTop = getPaddingTop(style);
  const padLeft = getPaddingLeft(style);
  const gap = getGap(style, isRow);

  const innerW = w - padH;
  const innerH = h - padV;

  // First pass: measure fixed-size children and count flex growers
  let usedSize = 0;
  let totalGrow = 0;
  const childInfos: Array<{ nodeId: number; style: NodeStyle; fixedSize: number | undefined; grow: number }> = [];

  for (const child of children) {
    const cs = child.nodeStyle;
    const grow = getFlexGrow(cs);
    const marginTop = getMarginTop(cs);

    let fixedSize: number | undefined;
    if (isRow) {
      fixedSize = resolveDim(cs.width, innerW);
    } else {
      fixedSize = resolveDim(cs.height, innerH);
      if (fixedSize !== undefined) fixedSize += marginTop;
    }

    if (fixedSize !== undefined) {
      usedSize += fixedSize;
    }

    if (grow > 0) {
      totalGrow += grow;
    } else if (fixedSize === undefined) {
      // Auto-sized: estimate as 1 row/col
      const est = isRow ? 0 : 1;
      usedSize += est;
      fixedSize = est;
    }

    childInfos.push({ nodeId: child.nodeId, style: cs, fixedSize, grow });
  }

  const totalGaps = gap * Math.max(0, children.length - 1);
  const mainAvail = isRow ? innerW : innerH;
  const remaining = Math.max(0, mainAvail - usedSize - totalGaps);

  // Second pass: assign positions
  let offset = isRow ? padLeft : padTop;

  for (const info of childInfos) {
    const cs = info.style;
    const grow = info.grow;
    const marginTop = isRow ? 0 : getMarginTop(cs);

    let childMainSize: number;
    if (grow > 0) {
      childMainSize = (info.fixedSize ?? 0) + Math.floor((grow / totalGrow) * remaining);
    } else {
      childMainSize = info.fixedSize ?? 0;
    }

    let childX: number;
    let childY: number;
    let childW: number;
    let childH: number;

    if (isRow) {
      childX = offset;
      childY = padTop;
      childW = childMainSize;
      childH = resolveDim(cs.height, innerH) ?? innerH;
    } else {
      childX = padLeft;
      childY = offset + marginTop;
      childW = resolveDim(cs.width, innerW) ?? innerW;
      childH = childMainSize - marginTop;
    }

    layoutNode(tree, info.nodeId, childX, childY, childW, childH, layouts);

    offset += childMainSize + gap;
    if (!isRow) offset += marginTop;
  }
};
