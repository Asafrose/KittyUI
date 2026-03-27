/**
 * Renderable — abstract base class for components that own a layout node.
 *
 * Every UI component extends Renderable and participates in the layout tree.
 * The Renderable manages its own node ID, style, text content, and lifecycle.
 */

import type { EncodedTextSpan } from "./mutation-encoder.js";
import { type CSSStyle, normalizeStyle } from "./style.js";
import type { ComputedLayout, NodeStyle, TextStyle } from "./types.js";

// ---------------------------------------------------------------------------
// ID generation
// ---------------------------------------------------------------------------

const INITIAL_NODE_ID = 1;
let nextNodeId = INITIAL_NODE_ID;

const allocNodeId = (): number => nextNodeId++;

/** Reset the ID counter (for testing). */
export const resetNodeIdCounter = (): void => {
  nextNodeId = INITIAL_NODE_ID;
};

// ---------------------------------------------------------------------------
// Renderable
// ---------------------------------------------------------------------------

export abstract class Renderable {
  /** Unique node ID for this component. */
  readonly nodeId: number;

  /** Normalized layout style. */
  private _nodeStyle: NodeStyle = {};

  /** Text style for this component's content. */
  private _textStyle: TextStyle = {};

  /** Text content, if any. */
  private _text: string | undefined;

  /** Computed layout from the last layout pass. */
  private _layout: ComputedLayout | undefined;

  /** Whether this node's style has been modified since last flush. */
  private _dirty = true;

  constructor() {
    this.nodeId = allocNodeId();
  }

  // -----------------------------------------------------------------------
  // Style
  // -----------------------------------------------------------------------

  /** Get the current layout style. */
  get nodeStyle(): NodeStyle {
    return this._nodeStyle;
  }

  /** Get the current text style. */
  get textStyle(): TextStyle {
    return this._textStyle;
  }

  /** Set style using CSS-like shorthand. Marks the node as dirty. */
  setStyle(css: CSSStyle): void {
    const { node, text } = normalizeStyle(css);
    this._nodeStyle = node;
    this._textStyle = text;
    this._dirty = true;
  }

  /** Update layout style directly. Marks the node as dirty. */
  setNodeStyle(style: NodeStyle): void {
    this._nodeStyle = style;
    this._dirty = true;
  }

  /** Update text style directly. */
  setTextStyle(style: TextStyle): void {
    this._textStyle = style;
  }

  // -----------------------------------------------------------------------
  // Text
  // -----------------------------------------------------------------------

  get text(): string | undefined {
    return this._text;
  }

  setText(text: string | undefined): void {
    this._text = text;
    this._dirty = true;
  }

  // -----------------------------------------------------------------------
  // Color spans (for inline colored text)
  // -----------------------------------------------------------------------

  /** Encoded color spans for per-character fg color overrides. */
  private _colorSpans: EncodedTextSpan[] = [];

  get colorSpans(): readonly EncodedTextSpan[] {
    return this._colorSpans;
  }

  setColorSpans(spans: EncodedTextSpan[]): void {
    this._colorSpans = spans;
    this._dirty = true;
  }

  // -----------------------------------------------------------------------
  // Layout
  // -----------------------------------------------------------------------

  get layout(): ComputedLayout | undefined {
    return this._layout;
  }

  /** Called by RenderableTree after layout computation. */
  updateLayout(layout: ComputedLayout): void {
    this._layout = layout;
  }

  // -----------------------------------------------------------------------
  // Dirty tracking
  // -----------------------------------------------------------------------

  get dirty(): boolean {
    return this._dirty;
  }

  clearDirty(): void {
    this._dirty = false;
  }

  markDirty(): void {
    this._dirty = true;
  }

  // -----------------------------------------------------------------------
  // Lifecycle hooks (override in subclasses)
  // -----------------------------------------------------------------------

  /** Called when the node is added to the tree. */
  onMount(): void {}

  /** Called when the node is removed from the tree. */
  onUnmount(): void {}

  /** Called after layout is computed. */
  onLayout(_layout: ComputedLayout): void {}
}
