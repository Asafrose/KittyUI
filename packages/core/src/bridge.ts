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

  /** Register a listener for events from the Rust engine. */
  onEvents(listener: (events: KittyEvent[]) => void): void {
    this.eventListeners.push(listener);
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
