/**
 * Bridge — manages the FFI lifecycle, mutation batching, and frame scheduling
 * between the TypeScript reconciler and the Rust core engine.
 */

import { FFIType, dlopen, ptr, suffix } from "bun:ffi";
import { existsSync } from "node:fs";
import { join } from "node:path";

import { MutationEncoder } from "./mutation-encoder.js";
import { EventDecoder } from "./event-decoder.js";
import type { KittyEvent } from "./event-decoder.js";

// -----------------------------------------------------------------------
// Native library loading
// -----------------------------------------------------------------------

const libPath = join(import.meta.dir, "..", "native", `libkittyui_core.${suffix}`);

const symbols = {
  hello: { args: [], returns: FFIType.cstring },
  init: { args: [FFIType.ptr], returns: FFIType.void },
  shutdown: { args: [], returns: FFIType.void },
  apply_mutations: { args: [FFIType.ptr, FFIType.u32], returns: FFIType.void },
  render_frame: { args: [], returns: FFIType.void },
  request_render: { args: [], returns: FFIType.void },
  start_render_loop: { args: [FFIType.u32], returns: FFIType.void },
  stop_render_loop: { args: [], returns: FFIType.void },
  register_event_callback: { args: [FFIType.function], returns: FFIType.void },
  get_layout: { args: [FFIType.u32, FFIType.ptr], returns: FFIType.void },
  get_all_layouts: { args: [FFIType.ptr, FFIType.u32], returns: FFIType.u32 },
  // Hit testing
  hit_test: {
    args: [FFIType.u16, FFIType.u16, FFIType.ptr, FFIType.u32],
    returns: FFIType.u32,
  },
  // Input system
  push_key_event: {
    args: [FFIType.u32, FFIType.u8, FFIType.u8],
    returns: FFIType.void,
  },
  push_mouse_event_with_hit_test: {
    args: [FFIType.u8, FFIType.u16, FFIType.u16, FFIType.u16, FFIType.u16, FFIType.u8],
    returns: FFIType.void,
  },
  focus: { args: [FFIType.u32], returns: FFIType.u8 },
  blur: { args: [], returns: FFIType.u8 },
  get_focused_node: { args: [], returns: FFIType.u32 },
  set_focusable: { args: [FFIType.u32, FFIType.u8], returns: FFIType.void },
  set_tab_index: { args: [FFIType.u32, FFIType.i32], returns: FFIType.void },
  set_focus_trap: { args: [FFIType.u32, FFIType.u8], returns: FFIType.void },
  set_viewport_size: { args: [FFIType.u16, FFIType.u16], returns: FFIType.void },
  get_terminal_caps: {
    args: [FFIType.ptr, FFIType.u32],
    returns: FFIType.u32,
  },
  set_pixel_size: { args: [FFIType.u32, FFIType.u32], returns: FFIType.void },
  set_cell_count: { args: [FFIType.u32, FFIType.u32], returns: FFIType.void },
  get_cell_pixel_width: { args: [], returns: FFIType.u32 },
  get_cell_pixel_height: { args: [], returns: FFIType.u32 },
} as const;

// -----------------------------------------------------------------------
// Result types
// -----------------------------------------------------------------------

/** Capabilities returned by the Rust `init()` call. */
export interface InitResult {
  versionMajor: number;
  versionMinor: number;
  versionPatch: number;
  batchedFfi: boolean;
}

/** Terminal capabilities detected at startup by the Rust engine. */
export interface TerminalCaps {
  kitty_graphics: boolean;
  true_color: boolean;
  cell_pixel_width: number;
  cell_pixel_height: number;
  terminal_name: string | null;
}

export interface NodeLayout {
  x: number;
  y: number;
  width: number;
  height: number;
}

// -----------------------------------------------------------------------
// Bridge class
// -----------------------------------------------------------------------

export class Bridge {
  private lib: ReturnType<typeof dlopen<typeof symbols>> | null = null;
  private encoder = new MutationEncoder();
  private decoder = new EventDecoder();
  private initialised = false;
  private eventListeners: ((events: KittyEvent[]) => void)[] = [];

  /** Whether the native library file exists on disk. */
  get nativeAvailable(): boolean {
    return existsSync(libPath);
  }

  /** Initialise the Rust engine. Returns the capabilities struct. */
  init(): InitResult {
    if (this.initialised) {
      throw new Error("Bridge already initialised");
    }
    if (!this.nativeAvailable) {
      throw new Error(
        "Native library not available — run `bun run build:native` first",
      );
    }
    this.lib = dlopen(libPath, symbols);
    // InitResult is #[repr(C)]: 3x u16 + 1x u8 = 7 bytes, padded to 8
    const STRUCT_SIZE = 8;
    const buf = new Uint8Array(STRUCT_SIZE);
    this.lib.symbols.init(ptr(buf));
    this.initialised = true;
    const view = new DataView(buf.buffer, buf.byteOffset, STRUCT_SIZE);
    return {
      batchedFfi: view.getUint8(6) !== 0,
      versionMajor: view.getUint16(0, true),
      versionMinor: view.getUint16(2, true),
      versionPatch: view.getUint16(4, true),
    };
  }

  /** Shut down the Rust engine and release resources. */
  shutdown(): void {
    if (!this.initialised || !this.lib) return;
    this.lib.symbols.shutdown();
    this.initialised = false;
    this.lib = null;
  }

  /** Get the mutation encoder for batching tree operations. */
  getEncoder(): MutationEncoder {
    return this.encoder;
  }

  /** Flush the current mutation batch to Rust. */
  flushMutations(): void {
    this.assertReady();
    if (this.encoder.byteLength === 0) return;
    const buf = this.encoder.toUint8Array();
    this.lib!.symbols.apply_mutations(buf, buf.byteLength);
    this.encoder.reset();
  }

  /** Run a full render frame (layout + render + diff + events). */
  renderFrame(): void {
    this.assertReady();
    this.lib!.symbols.render_frame();
  }

  /** Mark the scene as dirty for the next render tick. */
  requestRender(): void {
    this.assertReady();
    this.lib!.symbols.request_render();
  }

  /** Start an FPS-capped render loop on a Rust background thread. */
  startRenderLoop(fps = 60): void {
    this.assertReady();
    this.lib!.symbols.start_render_loop(fps);
  }

  /** Stop the background render loop. */
  stopRenderLoop(): void {
    this.assertReady();
    this.lib!.symbols.stop_render_loop();
  }

  /** Query the computed layout of a single node. */
  getLayout(nodeId: number): NodeLayout {
    this.assertReady();
    const buf = new Float32Array(4);
    this.lib!.symbols.get_layout(nodeId, buf);
    return {
      x: buf[0],
      y: buf[1],
      width: buf[2],
      height: buf[3],
    };
  }

  /** Query all computed layouts. Returns a Map of nodeId -> layout. */
  getAllLayouts(maxNodes = 1024): Map<number, NodeLayout> {
    this.assertReady();
    const buf = new Float32Array(maxNodes * 5);
    const count = this.lib!.symbols.get_all_layouts(buf, maxNodes);
    const result = new Map<number, NodeLayout>();
    for (let i = 0; i < count; i++) {
      const base = i * 5;
      result.set(buf[base], {
        x: buf[base + 1],
        y: buf[base + 2],
        width: buf[base + 3],
        height: buf[base + 4],
      });
    }
    return result;
  }

  /** Hit test at cell coordinates (x, y). Returns node IDs from deepest to root. */
  hitTest(x: number, y: number, maxDepth = 64): number[] {
    this.assertReady();
    const buf = new Uint32Array(maxDepth);
    const count = this.lib!.symbols.hit_test(x, y, ptr(buf), maxDepth);
    const result: number[] = [];
    for (let i = 0; i < count; i++) {
      result.push(buf[i]);
    }
    return result;
  }

  /** Register a listener for events from the Rust engine. */
  onEvents(listener: (events: KittyEvent[]) => void): void {
    this.eventListeners.push(listener);
  }

  /** Notify all registered event listeners with the given events. */
  notifyEventListeners(events: KittyEvent[]): void {
    for (const listener of this.eventListeners) {
      listener(events);
    }
  }

  // -----------------------------------------------------------------------
  // Input system
  // -----------------------------------------------------------------------

  /** Push a keyboard event into the engine's event buffer. */
  pushKeyEvent(keyCode: number, modifiers: number, eventType: number): void {
    this.assertReady();
    this.lib!.symbols.push_key_event(keyCode, modifiers, eventType);
  }

  /** Push a mouse event with automatic hit-testing. */
  pushMouseEvent(
    button: number,
    cellX: number,
    cellY: number,
    pixelX: number,
    pixelY: number,
    modifiers: number,
  ): void {
    this.assertReady();
    this.lib!.symbols.push_mouse_event_with_hit_test(
      button,
      cellX,
      cellY,
      pixelX,
      pixelY,
      modifiers,
    );
  }

  /** Focus a node by its id. Returns true if the node was focused. */
  focus(nodeId: number): boolean {
    this.assertReady();
    return this.lib!.symbols.focus(nodeId) !== 0;
  }

  /** Blur the currently focused node. Returns true if a node was blurred. */
  blur(): boolean {
    this.assertReady();
    return this.lib!.symbols.blur() !== 0;
  }

  /** Get the id of the currently focused node, or null if none. */
  getFocusedNode(): number | null {
    this.assertReady();
    const id = this.lib!.symbols.get_focused_node();
    return id === 0xffffffff ? null : id;
  }

  /** Mark a node as focusable or not. */
  setFocusable(nodeId: number, focusable: boolean): void {
    this.assertReady();
    this.lib!.symbols.set_focusable(nodeId, focusable ? 1 : 0);
  }

  /** Set the tab index for a node. */
  setTabIndex(nodeId: number, tabIndex: number): void {
    this.assertReady();
    this.lib!.symbols.set_tab_index(nodeId, tabIndex);
  }

  /** Enable or disable focus trapping on a node. */
  setFocusTrap(nodeId: number, enable: boolean): void {
    this.assertReady();
    this.lib!.symbols.set_focus_trap(nodeId, enable ? 1 : 0);
  }

  /** Set the viewport (terminal) size for layout computation. */
  setViewportSize(cols: number, rows: number): void {
    this.assertReady();
    this.lib!.symbols.set_viewport_size(cols, rows);
  }

  /** Query terminal capabilities detected at startup. */
  getCaps(): TerminalCaps {
    this.assertReady();
    const MAX_LEN = 1024;
    const buf = new Uint8Array(MAX_LEN);
    const n = this.lib!.symbols.get_terminal_caps(ptr(buf), MAX_LEN);
    const json = new TextDecoder().decode(buf.subarray(0, n));
    return JSON.parse(json) as TerminalCaps;
  }

  /** Store the terminal's total pixel dimensions (from CSI 14 t response). */
  setTerminalPixelSize(width: number, height: number): void {
    this.assertReady();
    this.lib!.symbols.set_pixel_size(width, height);
  }

  /** Store the terminal's cell count (from CSI 18 t response). */
  setTerminalCellCount(cols: number, rows: number): void {
    this.assertReady();
    this.lib!.symbols.set_cell_count(cols, rows);
  }

  /** Get the computed cell pixel width, or 0 if unknown. */
  getCellPixelWidth(): number {
    this.assertReady();
    return this.lib!.symbols.get_cell_pixel_width();
  }

  /** Get the computed cell pixel height, or 0 if unknown. */
  getCellPixelHeight(): number {
    this.assertReady();
    return this.lib!.symbols.get_cell_pixel_height();
  }

  /** Decode a raw event buffer and dispatch to listeners. */
  dispatchEvents(data: Uint8Array): void {
    const events = this.decoder.decode(data);
    for (const listener of this.eventListeners) {
      listener(events);
    }
  }

  private assertReady(): void {
    if (!this.initialised || !this.lib) {
      throw new Error("Bridge not initialised — call init() first");
    }
  }
}
