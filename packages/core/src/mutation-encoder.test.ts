import { describe, expect, test } from "bun:test";
import { MutationEncoder } from "./mutation-encoder.js";

describe("MutationEncoder", () => {
  test("createNode encodes op code, node id, and style JSON", () => {
    const enc = new MutationEncoder();
    enc.createNode(1, { width: 80, height: 24 });

    const data = enc.toUint8Array();
    const view = new DataView(data.buffer, data.byteOffset, data.byteLength);

    expect(view.getUint8(0)).toBe(1);
    expect(view.getUint32(1, true)).toBe(1);

    const jsonLen = view.getUint16(5, true);
    expect(jsonLen).toBeGreaterThan(0);

    const jsonBytes = data.slice(7, 7 + jsonLen);
    const json = new TextDecoder().decode(jsonBytes);
    const parsed = JSON.parse(json);
    expect(parsed.width).toBe(80);
    expect(parsed.height).toBe(24);
  });

  test("removeNode encodes op code and node id", () => {
    const enc = new MutationEncoder();
    enc.removeNode(42);

    const data = enc.toUint8Array();
    const view = new DataView(data.buffer, data.byteOffset, data.byteLength);

    expect(data.byteLength).toBe(5);
    expect(view.getUint8(0)).toBe(2);
    expect(view.getUint32(1, true)).toBe(42);
  });

  test("appendChild encodes parent and child ids", () => {
    const enc = new MutationEncoder();
    enc.appendChild(10, 20);

    const data = enc.toUint8Array();
    const view = new DataView(data.buffer, data.byteOffset, data.byteLength);

    expect(data.byteLength).toBe(9);
    expect(view.getUint8(0)).toBe(3);
    expect(view.getUint32(1, true)).toBe(10);
    expect(view.getUint32(5, true)).toBe(20);
  });

  test("insertBefore encodes parent, child, and before ids", () => {
    const enc = new MutationEncoder();
    enc.insertBefore(10, 20, 30);

    const data = enc.toUint8Array();
    const view = new DataView(data.buffer, data.byteOffset, data.byteLength);

    expect(data.byteLength).toBe(13);
    expect(view.getUint8(0)).toBe(4);
    expect(view.getUint32(1, true)).toBe(10);
    expect(view.getUint32(5, true)).toBe(20);
    expect(view.getUint32(9, true)).toBe(30);
  });

  test("setStyle encodes op, node id, and style JSON", () => {
    const enc = new MutationEncoder();
    enc.setStyle(5, { flexGrow: 1 });

    const data = enc.toUint8Array();
    const view = new DataView(data.buffer, data.byteOffset, data.byteLength);

    expect(view.getUint8(0)).toBe(5);
    expect(view.getUint32(1, true)).toBe(5);

    const jsonLen = view.getUint16(5, true);
    const json = new TextDecoder().decode(data.slice(7, 7 + jsonLen));
    expect(JSON.parse(json).flexGrow).toBe(1);
  });

  test("setText encodes op, node id, and text content", () => {
    const enc = new MutationEncoder();
    enc.setText(7, "Hello world");

    const data = enc.toUint8Array();
    const view = new DataView(data.buffer, data.byteOffset, data.byteLength);

    expect(view.getUint8(0)).toBe(6);
    expect(view.getUint32(1, true)).toBe(7);

    const textLen = view.getUint16(5, true);
    const text = new TextDecoder().decode(data.slice(7, 7 + textLen));
    expect(text).toBe("Hello world");
  });

  test("batching multiple mutations in one buffer", () => {
    const enc = new MutationEncoder();
    enc.createNode(1, { width: 80 });
    enc.createNode(2, { height: 10 });
    enc.appendChild(1, 2);

    const data = enc.toUint8Array();
    const view = new DataView(data.buffer, data.byteOffset, data.byteLength);

    expect(view.getUint8(0)).toBe(1);
    expect(view.getUint32(1, true)).toBe(1);
    expect(data.byteLength).toBeGreaterThan(20);
  });

  test("reset clears the buffer", () => {
    const enc = new MutationEncoder();
    enc.createNode(1, {});
    expect(enc.byteLength).toBeGreaterThan(0);

    enc.reset();
    expect(enc.byteLength).toBe(0);
  });

  test("auto-grows buffer when exceeding initial capacity", () => {
    const enc = new MutationEncoder(16);
    enc.createNode(1, { width: 100, height: 200, flexDirection: "column" });
    expect(enc.byteLength).toBeGreaterThan(16);
    expect(enc.toUint8Array()[0]).toBe(1);
  });

  test("handles unicode text in setText", () => {
    const enc = new MutationEncoder();
    enc.setText(1, "Hello \u{1F600}");

    const data = enc.toUint8Array();
    const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
    const textLen = view.getUint16(5, true);
    const text = new TextDecoder().decode(data.slice(7, 7 + textLen));
    expect(text).toBe("Hello \u{1F600}");
  });
});
