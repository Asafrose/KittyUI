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

const INITIAL_CAPACITY = 4096;

/**
 * Flatten a NodeStyle into the flat JSON format the Rust parser expects.
 * Converts Dim objects ({ type: "cells", value: N }) to plain numbers,
 * and extracts flex properties to top-level keys.
 */
const flattenStyle = (style: Record<string, unknown>): Record<string, unknown> => {
  const out: Record<string, unknown> = {};

  const flattenDim = (dim: unknown): number | undefined => {
    if (typeof dim === "number") return dim;
    if (dim && typeof dim === "object" && "type" in dim && "value" in dim) {
      const d = dim as { type: string; value: number };
      if (d.type === "cells") return d.value;
      // Rust parser doesn't support percent — approximate as cells
      // (the layout engine will handle percent natively if we extend the parser later)
    }
    return undefined;
  };

  // Dimension properties
  for (const key of ["width", "height", "minWidth", "minHeight", "maxWidth", "maxHeight"]) {
    if (style[key] !== undefined) {
      const v = flattenDim(style[key]);
      if (v !== undefined) out[key] = v;
    }
  }

  // Padding: [Dim, Dim, Dim, Dim] -> single number (use first value)
  if (Array.isArray(style.padding)) {
    const arr = style.padding as unknown[];
    const top = flattenDim(arr[0]);
    if (top !== undefined) out.padding = top;
  } else if (style.padding !== undefined) {
    const v = flattenDim(style.padding);
    if (v !== undefined) out.padding = v;
  }

  // Margin
  if (Array.isArray(style.margin)) {
    const arr = style.margin as unknown[];
    const top = flattenDim(arr[0]);
    if (top !== undefined) out.margin = top;
  } else if (style.margin !== undefined) {
    const v = flattenDim(style.margin);
    if (v !== undefined) out.margin = v;
  }

  // Gap
  if (Array.isArray(style.gap)) {
    const arr = style.gap as unknown[];
    const first = flattenDim(arr[0]);
    if (first !== undefined) out.gap = first;
  } else if (style.gap !== undefined) {
    const v = flattenDim(style.gap);
    if (v !== undefined) out.gap = v;
  }

  // Display mode — extract flex properties to top level
  if (style.display && typeof style.display === "object" && "type" in style.display) {
    const display = style.display as { type: string; flex?: Record<string, unknown> };
    if (display.type === "flex" && display.flex) {
      const flex = display.flex;
      if (flex.direction !== undefined) out.flexDirection = flex.direction;
      if (flex.grow !== undefined) out.flexGrow = flex.grow;
      if (flex.shrink !== undefined) out.flexShrink = flex.shrink;
      if (flex.basis !== undefined) {
        const v = flattenDim(flex.basis);
        if (v !== undefined) out.flexBasis = v;
      }
    }
  }

  // Pass through already-flat properties (for callers using raw style objects)
  for (const key of ["flexDirection", "flexGrow", "flexShrink", "flexBasis"]) {
    if (style[key] !== undefined && out[key] === undefined) {
      out[key] = style[key];
    }
  }

  return out;
};

export class MutationEncoder {
  private buffer: ArrayBuffer;
  private view: DataView;
  private offset: number;

  constructor(capacity = INITIAL_CAPACITY) {
    this.buffer = new ArrayBuffer(capacity);
    this.view = new DataView(this.buffer);
    this.offset = 0;
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
    const jsonBytes = new TextEncoder().encode(JSON.stringify(flattenStyle(style)));
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
    const jsonBytes = new TextEncoder().encode(JSON.stringify(flattenStyle(style)));
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
