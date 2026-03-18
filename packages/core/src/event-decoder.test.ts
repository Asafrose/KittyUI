import { describe, expect, test } from "bun:test";
import { EventDecoder } from "./event-decoder.js";

/** Helper: build a binary buffer from operations. */
function buildEventBuffer(
  ops: ((view: DataView, offset: number) => number)[],
): Uint8Array {
  const tempBuf = new ArrayBuffer(256);
  const tempView = new DataView(tempBuf);
  let totalSize = 0;
  for (const op of ops) {
    totalSize = op(tempView, totalSize);
  }
  return new Uint8Array(tempBuf, 0, totalSize);
}

function writeKeyboardEvent(
  keyCode: number,
  modifiers: number,
  eventType: number,
) {
  return (view: DataView, offset: number): number => {
    view.setUint8(offset, 1);
    offset += 1;
    view.setUint32(offset, keyCode, true);
    offset += 4;
    view.setUint8(offset, modifiers);
    offset += 1;
    view.setUint8(offset, eventType);
    offset += 1;
    return offset;
  };
}

function writeMouseEvent(
  button: number,
  x: number,
  y: number,
  pixelX: number,
  pixelY: number,
  modifiers: number,
  nodeId: number,
) {
  return (view: DataView, offset: number): number => {
    view.setUint8(offset, 2);
    offset += 1;
    view.setUint8(offset, button);
    offset += 1;
    view.setUint16(offset, x, true);
    offset += 2;
    view.setUint16(offset, y, true);
    offset += 2;
    view.setUint16(offset, pixelX, true);
    offset += 2;
    view.setUint16(offset, pixelY, true);
    offset += 2;
    view.setUint8(offset, modifiers);
    offset += 1;
    view.setUint32(offset, nodeId, true);
    offset += 4;
    return offset;
  };
}

function writeResizeEvent(
  cols: number,
  rows: number,
  pixelWidth: number,
  pixelHeight: number,
) {
  return (view: DataView, offset: number): number => {
    view.setUint8(offset, 3);
    offset += 1;
    view.setUint16(offset, cols, true);
    offset += 2;
    view.setUint16(offset, rows, true);
    offset += 2;
    view.setUint16(offset, pixelWidth, true);
    offset += 2;
    view.setUint16(offset, pixelHeight, true);
    offset += 2;
    return offset;
  };
}

function writeFocusEvent(nodeId: number) {
  return (view: DataView, offset: number): number => {
    view.setUint8(offset, 4);
    offset += 1;
    view.setUint32(offset, nodeId, true);
    offset += 4;
    return offset;
  };
}

function writeBlurEvent(nodeId: number) {
  return (view: DataView, offset: number): number => {
    view.setUint8(offset, 5);
    offset += 1;
    view.setUint32(offset, nodeId, true);
    offset += 4;
    return offset;
  };
}

describe("EventDecoder", () => {
  const decoder = new EventDecoder();

  test("decodes a keyboard event", () => {
    const data = buildEventBuffer([writeKeyboardEvent(65, 0b0000_0001, 1)]);
    const events = decoder.decode(data);

    expect(events).toHaveLength(1);
    expect(events[0].type).toBe("keyboard");
    if (events[0].type === "keyboard") {
      expect(events[0].keyCode).toBe(65);
      expect(events[0].modifiers).toBe(1);
      expect(events[0].eventType).toBe(1);
    }
  });

  test("decodes a mouse event", () => {
    const data = buildEventBuffer([writeMouseEvent(0, 10, 20, 80, 160, 0, 42)]);
    const events = decoder.decode(data);

    expect(events).toHaveLength(1);
    expect(events[0].type).toBe("mouse");
    if (events[0].type === "mouse") {
      expect(events[0].button).toBe(0);
      expect(events[0].x).toBe(10);
      expect(events[0].y).toBe(20);
      expect(events[0].pixelX).toBe(80);
      expect(events[0].pixelY).toBe(160);
      expect(events[0].modifiers).toBe(0);
      expect(events[0].nodeId).toBe(42);
    }
  });

  test("decodes a resize event", () => {
    const data = buildEventBuffer([writeResizeEvent(120, 40, 960, 640)]);
    const events = decoder.decode(data);

    expect(events).toHaveLength(1);
    expect(events[0].type).toBe("resize");
    if (events[0].type === "resize") {
      expect(events[0].cols).toBe(120);
      expect(events[0].rows).toBe(40);
      expect(events[0].pixelWidth).toBe(960);
      expect(events[0].pixelHeight).toBe(640);
    }
  });

  test("decodes multiple events in one buffer", () => {
    const data = buildEventBuffer([
      writeKeyboardEvent(65, 0, 1),
      writeMouseEvent(1, 5, 10, 40, 80, 2, 7),
      writeResizeEvent(80, 24, 640, 384),
    ]);
    const events = decoder.decode(data);

    expect(events).toHaveLength(3);
    expect(events[0].type).toBe("keyboard");
    expect(events[1].type).toBe("mouse");
    expect(events[2].type).toBe("resize");
  });

  test("returns empty array for empty buffer", () => {
    const data = new Uint8Array(0);
    const events = decoder.decode(data);
    expect(events).toHaveLength(0);
  });

  test("stops on unknown event type", () => {
    const buf = new Uint8Array([255]);
    const events = decoder.decode(buf);
    expect(events).toHaveLength(0);
  });

  test("stops on truncated keyboard event", () => {
    const buf = new Uint8Array([1, 65, 0, 0]);
    const events = decoder.decode(buf);
    expect(events).toHaveLength(0);
  });

  test("decodes a focus event", () => {
    const data = buildEventBuffer([writeFocusEvent(42)]);
    const events = decoder.decode(data);

    expect(events).toHaveLength(1);
    expect(events[0].type).toBe("focus");
    if (events[0].type === "focus") {
      expect(events[0].nodeId).toBe(42);
    }
  });

  test("decodes a blur event", () => {
    const data = buildEventBuffer([writeBlurEvent(7)]);
    const events = decoder.decode(data);

    expect(events).toHaveLength(1);
    expect(events[0].type).toBe("blur");
    if (events[0].type === "blur") {
      expect(events[0].nodeId).toBe(7);
    }
  });

  test("decodes focus and blur in sequence", () => {
    const data = buildEventBuffer([writeFocusEvent(5), writeBlurEvent(5)]);
    const events = decoder.decode(data);

    expect(events).toHaveLength(2);
    expect(events[0].type).toBe("focus");
    expect(events[1].type).toBe("blur");
  });

  test("decodes mixed events including focus/blur", () => {
    const data = buildEventBuffer([
      writeKeyboardEvent(65, 0, 1),
      writeFocusEvent(3),
      writeMouseEvent(1, 5, 10, 40, 80, 2, 7),
      writeBlurEvent(3),
    ]);
    const events = decoder.decode(data);

    expect(events).toHaveLength(4);
    expect(events[0].type).toBe("keyboard");
    expect(events[1].type).toBe("focus");
    expect(events[2].type).toBe("mouse");
    expect(events[3].type).toBe("blur");
  });

  test("stops on truncated focus event", () => {
    const buf = new Uint8Array([4, 42, 0]);
    const events = decoder.decode(buf);
    expect(events).toHaveLength(0);
  });
});
