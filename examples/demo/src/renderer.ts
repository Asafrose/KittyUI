/**
 * Simple TS-side ANSI renderer for the KittyUI demo.
 *
 * Reads computed layouts from either the Bridge (native) or the pure-TS
 * layout engine, and writes colored boxes/text to stdout using raw ANSI
 * escape sequences.
 */

import type { Bridge, Color, ComputedLayout, Renderable, RenderableTree } from "@kittyui/core";
import { computeLayouts } from "./layout.js";

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

/** Format a Color for debug output. */
const colorName = (color: Color): string => {
  if (color.type === "rgb") return `rgb(${color.r},${color.g},${color.b})`;
  if (color.type === "ansi") return `ansi(${color.index})`;
  if (color.type === "ansi-bright") return `ansi-bright(${color.index})`;
  if (color.type === "palette") return `palette(${color.index})`;
  return "unknown";
};

export interface RendererOptions {
  bridge?: Bridge;
  tree: RenderableTree;
  debug?: boolean;
}

export class DemoRenderer {
  private bridge: Bridge | undefined;
  private tree: RenderableTree;
  private debug: boolean;
  private cols: number;
  private rows: number;

  constructor(opts: RendererOptions) {
    this.bridge = opts.bridge;
    this.tree = opts.tree;
    this.debug = opts.debug ?? false;
    this.cols = process.stdout.columns || 80;
    this.rows = process.stdout.rows || 24;
  }

  /** Enter alternate screen and hide cursor (skipped in debug mode). */
  setup(): void {
    if (!this.debug) {
      process.stdout.write(enterAltScreen + hideCursor + clearScreen);
    }
    if (this.debug) {
      this.log(`Terminal dimensions: ${this.cols}x${this.rows}`);
      this.log(`Layout engine: ${this.bridge ? "native (Rust)" : "pure-TS"}`);
      this.log(`Tree size: ${this.tree.size} nodes`);
    }
    process.on("resize", () => {
      this.cols = process.stdout.columns || 80;
      this.rows = process.stdout.rows || 24;
      if (this.debug) {
        this.log(`Resize: ${this.cols}x${this.rows}`);
      }
    });
  }

  /** Exit alternate screen and show cursor (skipped in debug mode). */
  cleanup(): void {
    if (!this.debug) {
      process.stdout.write(resetStyle + showCursor + exitAltScreen);
    }
  }

  /** Write a debug message to stderr. */
  private log(msg: string): void {
    process.stderr.write(`[debug] ${msg}\n`);
  }

  /** Run one render frame: compute layout, then paint. */
  renderFrame(): void {
    let layouts: Map<number, ComputedLayout>;

    if (this.bridge) {
      // Native path: flush to Rust, compute, retrieve
      this.tree.flushDirtyStyles();
      this.bridge.flushMutations();
      this.bridge.renderFrame();
      layouts = this.bridge.getAllLayouts();
    } else {
      // Pure-TS path: compute layouts locally
      layouts = computeLayouts(this.tree, this.cols, this.rows);
    }

    this.tree.applyLayouts(layouts);

    if (this.debug) {
      this.log(`--- Frame (${layouts.size} layouts) ---`);
      this.logLayouts(layouts);
    }

    // Paint
    let buf = this.debug ? "" : clearScreen;
    const rootId = this.tree.root;
    if (rootId !== undefined) {
      buf += this.paintNode(rootId, 0, 0);
    }
    buf += resetStyle;
    if (!this.debug) {
      process.stdout.write(buf);
    }
  }

  /** Log all computed layouts to stderr. */
  private logLayouts(layouts: Map<number, ComputedLayout>): void {
    for (const [nodeId, layout] of layouts) {
      const renderable = this.tree.get(nodeId);
      const text = renderable?.text;
      const label = text ? ` text="${text.slice(0, 30)}${text.length > 30 ? "..." : ""}"` : "";
      this.log(
        `  node=${nodeId} x=${layout.x} y=${layout.y} w=${layout.width} h=${layout.height}${label}`,
      );
    }
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
      if (this.debug) {
        this.log(
          `  draw bg node=${nodeId} pos=(${absX},${absY}) size=${layout.width}x${layout.height} color=${colorName(bgColor)}`,
        );
      }
      buf += this.paintBackground(absX, absY, layout.width, layout.height, bgColor);
    }

    // Paint text content
    const text = renderable.text;
    if (text) {
      const fgColor = renderable.textStyle.fg;
      if (this.debug) {
        this.log(
          `  draw text node=${nodeId} pos=(${absX},${absY}) size=${layout.width}x${layout.height} fg=${fgColor ? colorName(fgColor) : "default"} text="${text.slice(0, 40)}${text.length > 40 ? "..." : ""}"`,
        );
      }
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
    const w = Math.min(Math.floor(width), this.cols - Math.floor(x));
    if (w <= 0) return "";
    const fill = " ".repeat(w);
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
    const maxWidth = Math.max(1, Math.floor(layout.width));
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
