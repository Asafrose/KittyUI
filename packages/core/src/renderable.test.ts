import { describe, expect, it, beforeEach } from "bun:test";
import { Renderable, resetNodeIdCounter } from "./renderable.js";
import type { ComputedLayout } from "./types.js";

class TestRenderable extends Renderable {
  mountCalled = false;
  unmountCalled = false;
  lastLayout: ComputedLayout | undefined;

  onMount(): void {
    this.mountCalled = true;
  }

  onUnmount(): void {
    this.unmountCalled = true;
  }

  onLayout(layout: ComputedLayout): void {
    this.lastLayout = layout;
  }
}

describe("Renderable", () => {
  beforeEach(() => {
    resetNodeIdCounter();
  });

  it("assigns unique node IDs", () => {
    const a = new TestRenderable();
    const b = new TestRenderable();
    expect(a.nodeId).toBe(1);
    expect(b.nodeId).toBe(2);
    expect(a.nodeId).not.toBe(b.nodeId);
  });

  it("starts dirty", () => {
    const r = new TestRenderable();
    expect(r.dirty).toBe(true);
  });

  it("clearDirty / markDirty", () => {
    const r = new TestRenderable();
    r.clearDirty();
    expect(r.dirty).toBe(false);
    r.markDirty();
    expect(r.dirty).toBe(true);
  });

  it("setStyle normalizes CSS and marks dirty", () => {
    const r = new TestRenderable();
    r.clearDirty();
    r.setStyle({ width: 40, color: "red" });
    expect(r.dirty).toBe(true);
    expect(r.nodeStyle.width).toEqual({ type: "cells", value: 40 });
    expect(r.textStyle.fg).toEqual({ type: "rgb", r: 255, g: 0, b: 0 });
  });

  it("setText marks dirty", () => {
    const r = new TestRenderable();
    r.clearDirty();
    r.setText("hello");
    expect(r.dirty).toBe(true);
    expect(r.text).toBe("hello");
  });

  it("updateLayout stores layout", () => {
    const r = new TestRenderable();
    const layout: ComputedLayout = { x: 1, y: 2, width: 10, height: 5 };
    r.updateLayout(layout);
    expect(r.layout).toEqual(layout);
  });

  it("setNodeStyle sets style directly", () => {
    const r = new TestRenderable();
    r.clearDirty();
    r.setNodeStyle({ width: { type: "cells", value: 50 } });
    expect(r.dirty).toBe(true);
    expect(r.nodeStyle.width).toEqual({ type: "cells", value: 50 });
  });

  it("setTextStyle sets text style directly", () => {
    const r = new TestRenderable();
    r.setTextStyle({ bold: true });
    expect(r.textStyle.bold).toBe(true);
  });
});
