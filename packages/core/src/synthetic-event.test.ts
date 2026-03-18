import { describe, expect, it } from "bun:test";
import {
  SyntheticKeyboardEvent,
  SyntheticMouseEvent,
  SyntheticResizeEvent,
  SyntheticFocusEvent,
} from "./synthetic-event.js";
import type { Modifiers } from "./types.js";

const NO_MODIFIERS: Modifiers = { shift: false, alt: false, ctrl: false, super: false };

describe("SyntheticEvent", () => {
  describe("SyntheticMouseEvent", () => {
    it("stores mouse event properties", () => {
      const event = new SyntheticMouseEvent({
        type: "mousedown",
        target: 5,
        button: "left",
        kind: "press",
        col: 10,
        row: 20,
        pixelX: 100,
        pixelY: 200,
        modifiers: NO_MODIFIERS,
      });
      expect(event.type).toBe("mousedown");
      expect(event.target).toBe(5);
      expect(event.currentTarget).toBe(5);
      expect(event.button).toBe("left");
      expect(event.kind).toBe("press");
      expect(event.col).toBe(10);
      expect(event.row).toBe(20);
      expect(event.pixelX).toBe(100);
      expect(event.pixelY).toBe(200);
    });

    it("stopPropagation works", () => {
      const event = new SyntheticMouseEvent({
        type: "click",
        target: 1,
        button: "left",
        kind: "release",
        col: 0,
        row: 0,
        pixelX: 0,
        pixelY: 0,
        modifiers: NO_MODIFIERS,
      });
      expect(event.isPropagationStopped).toBe(false);
      event.stopPropagation();
      expect(event.isPropagationStopped).toBe(true);
    });

    it("preventDefault works", () => {
      const event = new SyntheticMouseEvent({
        type: "click",
        target: 1,
        button: "left",
        kind: "release",
        col: 0,
        row: 0,
        pixelX: 0,
        pixelY: 0,
        modifiers: NO_MODIFIERS,
      });
      expect(event.isDefaultPrevented).toBe(false);
      event.preventDefault();
      expect(event.isDefaultPrevented).toBe(true);
    });

    it("has a timestamp", () => {
      const before = Date.now();
      const event = new SyntheticMouseEvent({
        type: "click",
        target: 1,
        button: "left",
        kind: "release",
        col: 0,
        row: 0,
        pixelX: 0,
        pixelY: 0,
        modifiers: NO_MODIFIERS,
      });
      const after = Date.now();
      expect(event.timestamp).toBeGreaterThanOrEqual(before);
      expect(event.timestamp).toBeLessThanOrEqual(after);
    });
  });

  describe("SyntheticKeyboardEvent", () => {
    it("stores keyboard event properties", () => {
      const event = new SyntheticKeyboardEvent({
        type: "keydown",
        target: 3,
        key: { type: "char", char: "a" },
        modifiers: { shift: true, alt: false, ctrl: false, super: false },
        eventType: "press",
      });
      expect(event.type).toBe("keydown");
      expect(event.target).toBe(3);
      expect(event.key).toEqual({ type: "char", char: "a" });
      expect(event.modifiers.shift).toBe(true);
      expect(event.eventType).toBe("press");
    });

    it("supports stopPropagation", () => {
      const event = new SyntheticKeyboardEvent({
        type: "keydown",
        target: 1,
        key: { type: "enter" },
        modifiers: NO_MODIFIERS,
        eventType: "press",
      });
      event.stopPropagation();
      expect(event.isPropagationStopped).toBe(true);
    });
  });

  describe("SyntheticResizeEvent", () => {
    it("stores resize properties", () => {
      const event = new SyntheticResizeEvent({
        target: 0,
        cols: 80,
        rows: 24,
        pixelWidth: 640,
        pixelHeight: 480,
      });
      expect(event.type).toBe("resize");
      expect(event.cols).toBe(80);
      expect(event.rows).toBe(24);
      expect(event.pixelWidth).toBe(640);
      expect(event.pixelHeight).toBe(480);
    });
  });

  describe("SyntheticFocusEvent", () => {
    it("stores focus type and target", () => {
      const focusEvent = new SyntheticFocusEvent("focus", 7);
      expect(focusEvent.type).toBe("focus");
      expect(focusEvent.target).toBe(7);

      const blurEvent = new SyntheticFocusEvent("blur", 7);
      expect(blurEvent.type).toBe("blur");
    });
  });

  describe("currentTarget mutation", () => {
    it("currentTarget can be updated during bubbling", () => {
      const event = new SyntheticMouseEvent({
        type: "click",
        target: 5,
        button: "left",
        kind: "release",
        col: 0,
        row: 0,
        pixelX: 0,
        pixelY: 0,
        modifiers: NO_MODIFIERS,
      });
      expect(event.currentTarget).toBe(5);
      event.currentTarget = 2;
      expect(event.currentTarget).toBe(2);
      expect(event.target).toBe(5);
    });
  });
});
