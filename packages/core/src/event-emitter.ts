/**
 * EventEmitter — a lightweight, typed event emitter for terminal events.
 *
 * Supports adding/removing listeners, one-shot listeners, and
 * emitting events to all registered handlers in order.
 */

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/** A listener callback. */
export type Listener<Payload> = (event: Payload) => void;

/** Constraint for event maps: string keys to any payload type. */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type EventMap = Record<string, any>;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const NOT_FOUND = -1;
const SPLICE_DELETE_COUNT = 1;
const EMPTY_COUNT = 0;

// ---------------------------------------------------------------------------
// EventEmitter
// ---------------------------------------------------------------------------

export class EventEmitter<Events extends EventMap> {
  private listeners = new Map<keyof Events, { fn: Listener<never>; once: boolean }[]>();

  /**
   * Register a listener for the given event.
   * Returns a dispose function to remove the listener.
   */
  on<EventKey extends keyof Events>(event: EventKey, fn: Listener<Events[EventKey]>): () => void {
    this.addEntry(event, fn as Listener<never>, false);
    return () => this.off(event, fn);
  }

  /**
   * Register a one-shot listener that auto-removes after the first call.
   */
  once<EventKey extends keyof Events>(event: EventKey, fn: Listener<Events[EventKey]>): () => void {
    this.addEntry(event, fn as Listener<never>, true);
    return () => this.off(event, fn);
  }

  /**
   * Remove a specific listener.
   */
  off<EventKey extends keyof Events>(event: EventKey, fn: Listener<Events[EventKey]>): void {
    const entries = this.listeners.get(event);
    if (!entries) {
      return;
    }
    const idx = entries.findIndex((entry) => entry.fn === fn);
    if (idx !== NOT_FOUND) {
      entries.splice(idx, SPLICE_DELETE_COUNT);
    }
    if (entries.length === EMPTY_COUNT) {
      this.listeners.delete(event);
    }
  }

  /**
   * Emit an event, calling all listeners in registration order.
   */
  emit<EventKey extends keyof Events>(event: EventKey, payload: Events[EventKey]): void {
    const entries = this.listeners.get(event);
    if (!entries) {
      return;
    }
    // Snapshot to allow mutations during iteration
    const snapshot = [...entries];
    for (const entry of snapshot) {
      if (entry.once) {
        this.off(event, entry.fn as Listener<Events[EventKey]>);
      }
      (entry.fn as Listener<Events[EventKey]>)(payload);
    }
  }

  /**
   * Remove all listeners, optionally for a specific event.
   */
  removeAllListeners<EventKey extends keyof Events>(event?: EventKey): void {
    if (event !== undefined) {
      this.listeners.delete(event);
    } else {
      this.listeners.clear();
    }
  }

  /**
   * Get the number of listeners for a given event.
   */
  listenerCount<EventKey extends keyof Events>(event: EventKey): number {
    return this.listeners.get(event)?.length ?? EMPTY_COUNT;
  }

  // -----------------------------------------------------------------------
  // Internal
  // -----------------------------------------------------------------------

  private addEntry<EventKey extends keyof Events>(
    event: EventKey,
    fn: Listener<never>,
    once: boolean,
  ): void {
    let entries = this.listeners.get(event);
    if (!entries) {
      entries = [];
      this.listeners.set(event, entries);
    }
    entries.push({ fn, once });
  }
}
