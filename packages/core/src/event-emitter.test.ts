import { describe, expect, it } from "bun:test";
import { EventEmitter } from "./event-emitter.js";

interface TestEvents {
  click: { x: number; y: number };
  resize: { cols: number; rows: number };
  close: void;
}

describe("EventEmitter", () => {
  it("calls listeners on emit", () => {
    const emitter = new EventEmitter<TestEvents>();
    const results: Array<{ x: number; y: number }> = [];
    emitter.on("click", (e) => results.push(e));
    emitter.emit("click", { x: 10, y: 20 });
    expect(results).toEqual([{ x: 10, y: 20 }]);
  });

  it("calls multiple listeners in order", () => {
    const emitter = new EventEmitter<TestEvents>();
    const order: number[] = [];
    emitter.on("click", () => order.push(1));
    emitter.on("click", () => order.push(2));
    emitter.on("click", () => order.push(3));
    emitter.emit("click", { x: 0, y: 0 });
    expect(order).toEqual([1, 2, 3]);
  });

  it("does not cross-emit between event types", () => {
    const emitter = new EventEmitter<TestEvents>();
    let clickCalled = false;
    let resizeCalled = false;
    emitter.on("click", () => { clickCalled = true; });
    emitter.on("resize", () => { resizeCalled = true; });
    emitter.emit("click", { x: 0, y: 0 });
    expect(clickCalled).toBe(true);
    expect(resizeCalled).toBe(false);
  });

  it("on() returns a dispose function", () => {
    const emitter = new EventEmitter<TestEvents>();
    let count = 0;
    const dispose = emitter.on("click", () => { count++; });
    emitter.emit("click", { x: 0, y: 0 });
    expect(count).toBe(1);
    dispose();
    emitter.emit("click", { x: 0, y: 0 });
    expect(count).toBe(1);
  });

  it("off() removes a listener", () => {
    const emitter = new EventEmitter<TestEvents>();
    let count = 0;
    const handler = () => { count++; };
    emitter.on("click", handler);
    emitter.off("click", handler);
    emitter.emit("click", { x: 0, y: 0 });
    expect(count).toBe(0);
  });

  it("off() is a no-op for unknown listener", () => {
    const emitter = new EventEmitter<TestEvents>();
    emitter.off("click", () => {});
    // Should not throw
  });

  it("once() fires only once", () => {
    const emitter = new EventEmitter<TestEvents>();
    let count = 0;
    emitter.once("click", () => { count++; });
    emitter.emit("click", { x: 0, y: 0 });
    emitter.emit("click", { x: 0, y: 0 });
    expect(count).toBe(1);
  });

  it("once() returns a dispose function that prevents firing", () => {
    const emitter = new EventEmitter<TestEvents>();
    let count = 0;
    const dispose = emitter.once("click", () => { count++; });
    dispose();
    emitter.emit("click", { x: 0, y: 0 });
    expect(count).toBe(0);
  });

  it("removeAllListeners() clears specific event", () => {
    const emitter = new EventEmitter<TestEvents>();
    let clickCount = 0;
    let resizeCount = 0;
    emitter.on("click", () => { clickCount++; });
    emitter.on("resize", () => { resizeCount++; });
    emitter.removeAllListeners("click");
    emitter.emit("click", { x: 0, y: 0 });
    emitter.emit("resize", { cols: 80, rows: 24 });
    expect(clickCount).toBe(0);
    expect(resizeCount).toBe(1);
  });

  it("removeAllListeners() with no arg clears everything", () => {
    const emitter = new EventEmitter<TestEvents>();
    let count = 0;
    emitter.on("click", () => { count++; });
    emitter.on("resize", () => { count++; });
    emitter.removeAllListeners();
    emitter.emit("click", { x: 0, y: 0 });
    emitter.emit("resize", { cols: 80, rows: 24 });
    expect(count).toBe(0);
  });

  it("listenerCount() returns correct count", () => {
    const emitter = new EventEmitter<TestEvents>();
    expect(emitter.listenerCount("click")).toBe(0);
    const dispose = emitter.on("click", () => {});
    emitter.on("click", () => {});
    expect(emitter.listenerCount("click")).toBe(2);
    dispose();
    expect(emitter.listenerCount("click")).toBe(1);
  });

  it("emit is safe when no listeners exist", () => {
    const emitter = new EventEmitter<TestEvents>();
    // Should not throw
    emitter.emit("click", { x: 0, y: 0 });
  });

  it("listener can add new listeners during emit", () => {
    const emitter = new EventEmitter<TestEvents>();
    let secondCalled = false;
    emitter.on("click", () => {
      emitter.on("click", () => { secondCalled = true; });
    });
    emitter.emit("click", { x: 0, y: 0 });
    // New listener should NOT be called during this emit (snapshot)
    expect(secondCalled).toBe(false);
    // But should fire on next emit
    emitter.emit("click", { x: 0, y: 0 });
    expect(secondCalled).toBe(true);
  });
});
