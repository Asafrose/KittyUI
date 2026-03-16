/**
 * EventDecoder — decodes the binary event buffer that Rust sends via
 * the registered event callback.
 *
 * Event format: [event_type: u8][payload...]
 *
 * Event types must stay in sync with `packages/core-rust/src/ffi_bridge.rs`.
 */

const EVENT_KEYBOARD = 1;
const EVENT_MOUSE = 2;
const EVENT_RESIZE = 3;

// -----------------------------------------------------------------------
// Event types
// -----------------------------------------------------------------------

export interface KeyboardEvent {
  type: "keyboard";
  keyCode: number;
  modifiers: number;
  eventType: number;
}

export interface MouseEvent {
  type: "mouse";
  button: number;
  x: number;
  y: number;
  pixelX: number;
  pixelY: number;
  modifiers: number;
  nodeId: number;
}

export interface ResizeEvent {
  type: "resize";
  cols: number;
  rows: number;
  pixelWidth: number;
  pixelHeight: number;
}

export type KittyEvent = KeyboardEvent | MouseEvent | ResizeEvent;

// -----------------------------------------------------------------------
// Decoder
// -----------------------------------------------------------------------

export class EventDecoder {
  /**
   * Decode a binary event buffer into an array of typed events.
   *
   * @param data - Uint8Array from the Rust event callback.
   */
  decode(data: Uint8Array): KittyEvent[] {
    const events: KittyEvent[] = [];
    const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
    let offset = 0;

    while (offset < data.byteLength) {
      const eventType = view.getUint8(offset);
      offset += 1;

      switch (eventType) {
        case EVENT_KEYBOARD: {
          if (offset + 6 > data.byteLength) return events;
          const keyCode = view.getUint32(offset, true);
          offset += 4;
          const modifiers = view.getUint8(offset);
          offset += 1;
          const evtType = view.getUint8(offset);
          offset += 1;
          events.push({
            type: "keyboard",
            keyCode,
            modifiers,
            eventType: evtType,
          });
          break;
        }
        case EVENT_MOUSE: {
          if (offset + 14 > data.byteLength) return events;
          const button = view.getUint8(offset);
          offset += 1;
          const x = view.getUint16(offset, true);
          offset += 2;
          const y = view.getUint16(offset, true);
          offset += 2;
          const pixelX = view.getUint16(offset, true);
          offset += 2;
          const pixelY = view.getUint16(offset, true);
          offset += 2;
          const modifiers = view.getUint8(offset);
          offset += 1;
          const nodeId = view.getUint32(offset, true);
          offset += 4;
          events.push({
            type: "mouse",
            button,
            x,
            y,
            pixelX,
            pixelY,
            modifiers,
            nodeId,
          });
          break;
        }
        case EVENT_RESIZE: {
          if (offset + 8 > data.byteLength) return events;
          const cols = view.getUint16(offset, true);
          offset += 2;
          const rows = view.getUint16(offset, true);
          offset += 2;
          const pixelWidth = view.getUint16(offset, true);
          offset += 2;
          const pixelHeight = view.getUint16(offset, true);
          offset += 2;
          events.push({
            type: "resize",
            cols,
            rows,
            pixelWidth,
            pixelHeight,
          });
          break;
        }
        default:
          // Unknown event — stop decoding
          return events;
      }
    }

    return events;
  }
}
