/**
 * Simple TS-side ANSI renderer for the KittyUI demo.
 *
 * Reads computed layouts from the Bridge and writes colored boxes/text
 * to stdout using raw ANSI escape sequences.
 */

import type { Bridge, Color, ComputedLayout, Renderable, RenderableTree } from "@kittyui/core";

// ---------------------------------------------------------------------------
// ANSI helpers
// ---------------------------------------------------------------------------

const ESC = "\x1b[";

const cursorTo = (row: number, col: number): string =>
  `${ESC}${Math.floor(row) + 1};${Math.floor(col) + 1}H`;

const hideCursor = `${ESC}?25l`;
const showCursor = `${ESC}?25h`;
const enterAltScreen = `${ESC}?1049h`;
const exitAltScreen = `${ESC}?1049l`;
const resetStyle = `${ESC}0m`;
const clearScreen = `${ESC}2J`;

const colorToAnsiFg = (color: Color): string => {
  if (color.type === "rgb") {
    return `${ESC}38;2;${color.r};${color.g};${color.b}m`;
  }
  if (color.type === "ansi") {
    return `${ESC}${30 + color.index}m`;
  }
  if (color.type === "ansi-bright") {
    return `${ESC}${90 + color.index}m`;
  }
  if (color.type === "palette") {
    return `${ESC}38;5;${color.index}m`;
  }
  return "";
};

const colorToAnsiBg = (color: Color): string => {
  if (color.type === "rgb") {
    return `${ESC}48;2;${color.r};${color.g};${color.b}m`;
  }
  if (color.type === "ansi") {
    return `${ESC}${40 + color.index}m`;
  }
  if (color.type === "ansi-bright") {
    return `${ESC}${100 + color.index}m`;
  }
  if (color.type === "palette") {
    return `${ESC}48;5;${color.index}m`;
  }
  return "";
};

// ---------------------------------------------------------------------------
// Renderer
// ---------------------------------------------------------------------------

export interface RendererOptions {
  bridge: Bridge;
  tree: RenderableTree;
}

export class DemoRenderer {
  private bridge: Bridge;
  private tree: RenderableTree;
  private cols: number;
  private rows: number;

  constructor(opts: RendererOptions) {
    this.bridge = opts.bridge;
    this.tree = opts.tree;
    this.cols = process.stdout.columns || 80;
    this.rows = process.stdout.rows || 24;
  }

  /** Enter alternate screen and hide cursor. */
  setup(): void {
    process.stdout.write(enterAltScreen + hideCursor + clearScreen);
    process.on("resize", () => {
      this.cols = process.stdout.columns || 80;
      this.rows = process.stdout.rows || 24;
    });
  }

  /** Exit alternate screen and show cursor. */
  cleanup(): void {
    process.stdout.write(resetStyle + showCursor + exitAltScreen);
  }

  /** Run one render frame: compute layout, then paint. */
  renderFrame(): void {
    // Flush any pending style changes
    this.tree.flushDirtyStyles();
    this.bridge.flushMutations();

    // Compute layout on the Rust side
    this.bridge.renderFrame();

    // Get all computed layouts
    const layouts = this.bridge.getAllLayouts();
    this.tree.applyLayouts(layouts);

    // Paint
    let buf = clearScreen;
    const rootId = this.tree.root;
    if (rootId !== undefined) {
      buf += this.paintNode(rootId, 0, 0);
    }
    buf += resetStyle;
    process.stdout.write(buf);
  }

  private paintNode(nodeId: number, offsetX: number, offsetY: number): string {
    const renderable = this.tree.get(nodeId);
    if (!renderable) return "";

    const layout = renderable.layout;
    if (!layout) return "";

    const absX = offsetX + layout.x;
    const absY = offsetY + layout.y;

    let buf = "";

    // Paint background
    const bgColor = renderable.textStyle.bg;
    if (bgColor) {
      buf += this.paintBackground(absX, absY, layout.width, layout.height, bgColor);
    }

    // Paint text content
    const text = renderable.text;
    if (text) {
      buf += this.paintText(absX, absY, layout, renderable);
    }

    // Paint children
    const children = this.tree.children(nodeId);
    for (const child of children) {
      buf += this.paintNode(child.nodeId, absX, absY);
    }

    return buf;
  }

  private paintBackground(
    x: number,
    y: number,
    width: number,
    height: number,
    color: Color,
  ): string {
    let buf = colorToAnsiBg(color);
    const fill = " ".repeat(Math.max(0, Math.floor(width)));
    for (let row = 0; row < Math.floor(height); row++) {
      const screenRow = y + row;
      if (screenRow < 0 || screenRow >= this.rows) continue;
      const startCol = Math.max(0, Math.floor(x));
      if (startCol >= this.cols) continue;
      buf += cursorTo(screenRow, startCol) + fill;
    }
    buf += resetStyle;
    return buf;
  }

  private paintText(
    x: number,
    y: number,
    layout: ComputedLayout,
    renderable: Renderable,
  ): string {
    const text = renderable.text;
    if (!text) return "";

    const style = renderable.textStyle;
    let buf = "";

    // Apply text style
    if (style.fg) buf += colorToAnsiFg(style.fg);
    if (style.bg) buf += colorToAnsiBg(style.bg);
    if (style.bold) buf += `${ESC}1m`;
    if (style.dim) buf += `${ESC}2m`;
    if (style.italic) buf += `${ESC}3m`;
    if (style.underline) buf += `${ESC}4m`;

    // Render text lines, wrapping to fit layout width
    const maxWidth = Math.floor(layout.width);
    const lines = this.wrapText(text, maxWidth);

    for (let i = 0; i < lines.length && i < Math.floor(layout.height); i++) {
      const screenRow = y + i;
      if (screenRow < 0 || screenRow >= this.rows) continue;
      const screenCol = Math.max(0, Math.floor(x));
      if (screenCol >= this.cols) continue;
      const line = lines[i].slice(0, this.cols - screenCol);
      buf += cursorTo(screenRow, screenCol) + line;
    }

    buf += resetStyle;
    return buf;
  }

  private wrapText(text: string, maxWidth: number): string[] {
    if (maxWidth <= 0) return [];
    const lines: string[] = [];
    // Split on newlines first
    for (const segment of text.split("\n")) {
      if (segment.length <= maxWidth) {
        lines.push(segment);
      } else {
        // Word-wrap
        let pos = 0;
        while (pos < segment.length) {
          lines.push(segment.slice(pos, pos + maxWidth));
          pos += maxWidth;
        }
      }
    }
    return lines;
  }
}
