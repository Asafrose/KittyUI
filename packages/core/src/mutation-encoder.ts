/**
 * MutationEncoder — encodes React reconciler tree mutations into a binary
 * buffer that Rust's `apply_mutations()` can decode in a single FFI call.
 *
 * Binary format per op:
 *   [op_code: u8][node_id: u32][payload...]
 *
 * Op codes must stay in sync with `packages/core-rust/src/ffi_bridge.rs`.
 */

// Op codes — keep in sync with Rust OP_* constants.
const OP_CREATE_NODE = 1;
const OP_REMOVE_NODE = 2;
const OP_APPEND_CHILD = 3;
const OP_INSERT_BEFORE = 4;
const OP_SET_STYLE = 5;
const OP_SET_TEXT = 6;
const OP_SET_TEXT_SPANS = 7;

/** A single color span for setTextSpans. */
export interface EncodedTextSpan {
  start: number;
  end: number;
  r: number;
  g: number;
  b: number;
}

const BYTES_PER_SPAN = 7;

const INITIAL_CAPACITY = 4096;
const PERCENT_DIVISOR = 100;

// ---------------------------------------------------------------------------
// Style preprocessing — flatten structured Dim / display objects for Rust
// ---------------------------------------------------------------------------

/** Resolve a Dim value to a plain number. */
const resolveDim = (dim: unknown, termSize: number): number | undefined => {
  if (dim === undefined || dim === null) return undefined;  // eslint-disable-line unicorn/no-null, eqeqeq, no-eq-null
  if (typeof dim === "number") return dim;
  if (typeof dim === "string") {
    const trimmed = dim.trim();
    if (trimmed.endsWith("%")) {
      const pct = Number.parseFloat(trimmed);
      if (!Number.isNaN(pct)) return Math.floor((pct / PERCENT_DIVISOR) * termSize);
    }
    const n = Number.parseFloat(trimmed);
    if (!Number.isNaN(n)) return n;
    return undefined;
  }
  if (typeof dim === "object") {
    const d = dim as Record<string, unknown>;
    if (d.type === "cells") return d.value as number;
    if (d.type === "percent") {
      return Math.floor(((d.value as number) / PERCENT_DIVISOR) * termSize);
    }
    if (d.type === "auto") return undefined;
  }
  return undefined;
};

/** Flatten a padding/margin/gap Dim array to a plain number or array of numbers. */
const flattenDimArray = (arr: unknown, termSize: number): unknown => {
  if (!Array.isArray(arr)) return resolveDim(arr, termSize);
  return arr.map((item) => resolveDim(item, termSize) ?? 0);
};

/** Preprocess a style record so Rust receives only flat primitives. */
const preprocessStyle = (style: Record<string, unknown>, overrideCols?: number, overrideRows?: number): Record<string, unknown> => {
  const result: Record<string, unknown> = {};
  const cols = overrideCols ?? ((typeof process !== "undefined" && process.stdout?.columns) || 80);
  const rows = overrideRows ?? ((typeof process !== "undefined" && process.stdout?.rows) || 24);

  for (const [key, value] of Object.entries(style)) {
    if (value === undefined) continue;

    switch (key) {
      case "width":
      case "minWidth":
      case "maxWidth":
      case "flexBasis": {
        const resolved = resolveDim(value, cols);
        if (resolved !== undefined) result[key] = resolved;
        break;
      }
      case "height":
      case "minHeight":
      case "maxHeight": {
        const resolved = resolveDim(value, rows);
        if (resolved !== undefined) result[key] = resolved;
        break;
      }
      case "padding":
      case "margin": {
        const flat = flattenDimArray(value, cols);
        if (flat !== undefined) result[key] = flat;
        break;
      }
      case "gap": {
        const flat = flattenDimArray(value, cols);
        if (flat !== undefined) result[key] = flat;
        break;
      }
      case "display": {
        // Extract flex/grid properties into top-level keys.
        if (typeof value === "object" && value !== null) {  // eslint-disable-line unicorn/no-null, eqeqeq, no-eq-null
          const d = value as Record<string, unknown>;
          if (d.type === "flex" && typeof d.flex === "object" && d.flex !== null) {  // eslint-disable-line unicorn/no-null, eqeqeq, no-eq-null
            const flex = d.flex as Record<string, unknown>;
            if (flex.direction !== undefined) result.flexDirection = flex.direction;
            if (flex.grow !== undefined) result.flexGrow = flex.grow;
            if (flex.shrink !== undefined) result.flexShrink = flex.shrink;
            if (flex.wrap !== undefined) result.flexWrap = flex.wrap;
            if (flex.justify !== undefined) result.justifyContent = flex.justify;
            if (flex.alignItems !== undefined) result.alignItems = flex.alignItems;
            if (flex.basis !== undefined) {
              const resolved = resolveDim(flex.basis, cols);
              if (resolved !== undefined) result.flexBasis = resolved;
            }
          }
        }
        break;
      }
      case "background":
      case "backgroundColor":
      case "color": {
        // Pass through string values.
        result[key] = value;
        break;
      }
      case "flexGrow":
      case "flexShrink":
      case "flexDirection":
      case "flexWrap":
      case "justifyContent":
      case "alignItems":
      case "bold":
      case "italic":
      case "textOverflow":
      case "underline":
      case "strikethrough":
      case "dim":
      case "textDecoration":
      case "overflow":
      case "border":
      case "borderColor":
      case "boxShadow":
      case "borderRadius": {
        result[key] = value;
        break;
      }
      case "paddingTop":
      case "paddingRight":
      case "paddingBottom":
      case "paddingLeft":
      case "marginTop":
      case "marginRight":
      case "marginBottom":
      case "marginLeft": {
        const resolved = resolveDim(value, cols);
        if (resolved !== undefined) result[key] = resolved;
        break;
      }
      default:
        break;
    }
  }

  return result;
};

export class MutationEncoder {
  private buffer: ArrayBuffer;
  private view: DataView;
  private offset: number;
  private overrideCols: number | undefined;
  private overrideRows: number | undefined;

  constructor(capacity = INITIAL_CAPACITY) {
    this.buffer = new ArrayBuffer(capacity);
    this.view = new DataView(this.buffer);
    this.offset = 0;
  }

  /** Set the viewport size for percentage resolution (used in test mode). */
  setViewportSize(cols: number, rows: number): void {
    this.overrideCols = cols;
    this.overrideRows = rows;
  }

  /** Reset the encoder for a new batch of mutations. */
  reset(): void {
    this.offset = 0;
  }

  /** Number of bytes written so far. */
  get byteLength(): number {
    return this.offset;
  }

  /** Return a Uint8Array view over the written bytes. */
  toUint8Array(): Uint8Array {
    return new Uint8Array(this.buffer, 0, this.offset);
  }

  /** Return the raw ArrayBuffer pointer for FFI. */
  get ptr(): ArrayBuffer {
    return this.buffer;
  }

  // -----------------------------------------------------------------------
  // Mutation ops
  // -----------------------------------------------------------------------

  createNode(nodeId: number, style: Record<string, unknown>): void {
    const jsonBytes = new TextEncoder().encode(JSON.stringify(preprocessStyle(style, this.overrideCols, this.overrideRows)));
    this.ensureCapacity(1 + 4 + 2 + jsonBytes.byteLength);
    this.writeU8(OP_CREATE_NODE);
    this.writeU32(nodeId);
    this.writeU16(jsonBytes.byteLength);
    this.writeBytes(jsonBytes);
  }

  removeNode(nodeId: number): void {
    this.ensureCapacity(1 + 4);
    this.writeU8(OP_REMOVE_NODE);
    this.writeU32(nodeId);
  }

  appendChild(parentId: number, childId: number): void {
    this.ensureCapacity(1 + 4 + 4);
    this.writeU8(OP_APPEND_CHILD);
    this.writeU32(parentId);
    this.writeU32(childId);
  }

  insertBefore(parentId: number, childId: number, beforeId: number): void {
    this.ensureCapacity(1 + 4 + 4 + 4);
    this.writeU8(OP_INSERT_BEFORE);
    this.writeU32(parentId);
    this.writeU32(childId);
    this.writeU32(beforeId);
  }

  setStyle(nodeId: number, style: Record<string, unknown>): void {
    const jsonBytes = new TextEncoder().encode(JSON.stringify(preprocessStyle(style, this.overrideCols, this.overrideRows)));
    this.ensureCapacity(1 + 4 + 2 + jsonBytes.byteLength);
    this.writeU8(OP_SET_STYLE);
    this.writeU32(nodeId);
    this.writeU16(jsonBytes.byteLength);
    this.writeBytes(jsonBytes);
  }

  setText(nodeId: number, text: string): void {
    const textBytes = new TextEncoder().encode(text);
    this.ensureCapacity(1 + 4 + 2 + textBytes.byteLength);
    this.writeU8(OP_SET_TEXT);
    this.writeU32(nodeId);
    this.writeU16(textBytes.byteLength);
    this.writeBytes(textBytes);
  }

  /**
   * Send per-span fg color data for a text node.
   *
   * Binary format:
   *   [OP_SET_TEXT_SPANS:u8][node_id:u32][span_count:u16]
   *   per span: [start:u16][end:u16][r:u8][g:u8][b:u8]
   */
  setTextSpans(nodeId: number, spans: readonly EncodedTextSpan[]): void {
    this.ensureCapacity(1 + 4 + 2 + spans.length * BYTES_PER_SPAN);
    this.writeU8(OP_SET_TEXT_SPANS);
    this.writeU32(nodeId);
    this.writeU16(spans.length);
    for (const span of spans) {
      this.writeU16(span.start);
      this.writeU16(span.end);
      this.writeU8(span.r);
      this.writeU8(span.g);
      this.writeU8(span.b);
    }
  }

  // -----------------------------------------------------------------------
  // Low-level writers
  // -----------------------------------------------------------------------

  private writeU8(value: number): void {
    this.view.setUint8(this.offset, value);
    this.offset += 1;
  }

  private writeU16(value: number): void {
    this.view.setUint16(this.offset, value, true);
    this.offset += 2;
  }

  private writeU32(value: number): void {
    this.view.setUint32(this.offset, value, true);
    this.offset += 4;
  }

  private writeBytes(bytes: Uint8Array): void {
    new Uint8Array(this.buffer, this.offset, bytes.byteLength).set(bytes);
    this.offset += bytes.byteLength;
  }

  private ensureCapacity(additional: number): void {
    const needed = this.offset + additional;
    if (needed <= this.buffer.byteLength) {
      return;
    }
    let newSize = this.buffer.byteLength;
    while (newSize < needed) {
      newSize *= 2;
    }
    const newBuf = new ArrayBuffer(newSize);
    new Uint8Array(newBuf).set(new Uint8Array(this.buffer, 0, this.offset));
    this.buffer = newBuf;
    this.view = new DataView(this.buffer);
  }
}
