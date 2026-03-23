/**
 * TestBridge — extends Bridge functionality for headless testing.
 *
 * Uses init_test_mode / get_rendered_output / shutdown_test_mode instead of
 * the normal init/shutdown that manipulate the real terminal.
 */

import { FFIType, dlopen, ptr, suffix } from "bun:ffi";
import { existsSync } from "node:fs";
import { join } from "node:path";

import { MutationEncoder } from "../mutation-encoder.js";
import type { InitResult } from "../bridge.js";

// -----------------------------------------------------------------------
// Native library loading
// -----------------------------------------------------------------------

const libPath = join(import.meta.dir, "..", "..", "native", `libkittyui_core.${suffix}`);

const testSymbols = {
  apply_mutations: { args: [FFIType.ptr, FFIType.u32], returns: FFIType.void },
  get_all_layouts: { args: [FFIType.ptr, FFIType.u32], returns: FFIType.u32 },
  get_layout: { args: [FFIType.u32, FFIType.ptr], returns: FFIType.void },
  get_rendered_output: { args: [FFIType.ptr, FFIType.u32], returns: FFIType.u32 },
  init_test_mode: { args: [FFIType.u16, FFIType.u16, FFIType.ptr], returns: FFIType.void },
  render_frame: { args: [], returns: FFIType.void },
  shutdown_test_mode: { args: [], returns: FFIType.void },
} as const;

// -----------------------------------------------------------------------
// TestBridge
// -----------------------------------------------------------------------

export interface NodeLayout {
  x: number;
  y: number;
  width: number;
  height: number;
}

export class TestBridge {
  private lib: ReturnType<typeof dlopen<typeof testSymbols>> | null = null;
  private encoder = new MutationEncoder();
  private initialised = false;

  /** Whether the native library file exists on disk. */
  get nativeAvailable(): boolean {
    return existsSync(libPath);
  }

  /** Get the mutation encoder. */
  getEncoder(): MutationEncoder {
    return this.encoder;
  }

  /** Initialise the engine in test mode (no terminal side effects). */
  initTestMode(cols: number, rows: number): InitResult {
    if (this.initialised) {
      throw new Error("TestBridge already initialised");
    }
    if (!this.nativeAvailable) {
      throw new Error("Native library not available — run `bun run build:native` first");
    }
    this.lib = dlopen(libPath, testSymbols);

    const STRUCT_SIZE = 8;
    const buf = new Uint8Array(STRUCT_SIZE);
    this.lib.symbols.init_test_mode(cols, rows, ptr(buf));
    this.initialised = true;

    const view = new DataView(buf.buffer, buf.byteOffset, STRUCT_SIZE);
    return {
      batchedFfi: view.getUint8(6) !== 0,
      versionMajor: view.getUint16(0, true),
      versionMinor: view.getUint16(2, true),
      versionPatch: view.getUint16(4, true),
    };
  }

  /** Flush mutations to the Rust engine. */
  flushMutations(): void {
    this.assertReady();
    if (this.encoder.byteLength === 0) return;
    const buf = this.encoder.toUint8Array();
    this.lib!.symbols.apply_mutations(buf, buf.byteLength);
    this.encoder.reset();
  }

  /** Run a full render frame. */
  renderFrame(): void {
    this.assertReady();
    this.lib!.symbols.render_frame();
  }

  /** Get the rendered ANSI output and clear the internal buffer. */
  getRenderedOutput(): Uint8Array {
    this.assertReady();
    const MAX_OUTPUT = 65536;
    const buf = new Uint8Array(MAX_OUTPUT);
    const n = this.lib!.symbols.get_rendered_output(ptr(buf), MAX_OUTPUT);
    return buf.slice(0, n);
  }

  /** Query the computed layout of a single node. */
  getLayout(nodeId: number): NodeLayout {
    this.assertReady();
    const buf = new Float32Array(4);
    this.lib!.symbols.get_layout(nodeId, buf);
    return {
      height: buf[3],
      width: buf[2],
      x: buf[0],
      y: buf[1],
    };
  }

  /** Query all computed layouts. */
  getAllLayouts(maxNodes = 1024): Map<number, NodeLayout> {
    this.assertReady();
    const buf = new Float32Array(maxNodes * 5);
    const count = this.lib!.symbols.get_all_layouts(buf, maxNodes);
    const result = new Map<number, NodeLayout>();
    for (let i = 0; i < count; i++) {
      const base = i * 5;
      result.set(buf[base], {
        height: buf[base + 4],
        width: buf[base + 3],
        x: buf[base + 1],
        y: buf[base + 2],
      });
    }
    return result;
  }

  /** Shut down the engine in test mode (no terminal side effects). */
  shutdownTestMode(): void {
    if (!this.initialised || !this.lib) return;
    this.lib.symbols.shutdown_test_mode();
    this.initialised = false;
    this.lib = null;
  }

  private assertReady(): void {
    if (!this.initialised || !this.lib) {
      throw new Error("TestBridge not initialised — call initTestMode() first");
    }
  }
}
