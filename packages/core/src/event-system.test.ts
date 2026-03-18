import { describe, expect, it, beforeEach } from "bun:test";
import { Renderable, resetNodeIdCounter } from "./renderable.js";
import { RenderableTree } from "./renderable-tree.js";
import { MutationEncoder } from "./mutation-encoder.js";
import { EventSystem } from "./event-system.js";
import type { SyntheticMouseEvent } from "./synthetic-event.js";
import type { SyntheticKeyboardEvent } from "./synthetic-event.js";
import type { SyntheticFocusEvent } from "./synthetic-event.js";
import type { SyntheticResizeEvent } from "./synthetic-event.js";
import type { HitResult, Modifiers } from "./types.js";
import type { DispatchMouseOptions } from "./event-system.js";

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

class TestRenderable extends Renderable {}

const NO_MODIFIERS: Modifiers = { shift: false, alt: false, ctrl: false, super: false };

const mouseOpts = (overrides: Partial<DispatchMouseOptions> = {}): DispatchMouseOptions => ({
  button: "left",
  kind: "press",
  col: 8,
  row: 8,
  pixelX: 80,
  pixelY: 80,
  modifiers: NO_MODIFIERS,
  ...overrides,
});

/** Create a simple tree: root -> child -> grandchild, with layouts assigned. */
const buildTestTree = () => {
  const encoder = new MutationEncoder();
  const tree = new RenderableTree(encoder);

  const root = new TestRenderable();
  const child = new TestRenderable();
  const grandchild = new TestRenderable();

  tree.setRoot(root);
  tree.appendChild(root.nodeId, child);
  tree.appendChild(child.nodeId, grandchild);

  root.updateLayout({ x: 0, y: 0, width: 80, height: 24 });
  child.updateLayout({ x: 5, y: 5, width: 20, height: 10 });
  grandchild.updateLayout({ x: 7, y: 7, width: 10, height: 5 });

  return { tree, root, child, grandchild };
};

describe("EventSystem", () => {
  let tree: RenderableTree;
  let root: TestRenderable;
  let child: TestRenderable;
  let grandchild: TestRenderable;
  let eventSystem: EventSystem;

  beforeEach(() => {
    resetNodeIdCounter();
    const built = buildTestTree();
    tree = built.tree;
    root = built.root;
    child = built.child;
    grandchild = built.grandchild;

    const defaultHitTest = (_col: number, _row: number): HitResult | undefined => ({
      target: grandchild.nodeId,
      path: [grandchild.nodeId, child.nodeId, root.nodeId],
    });

    eventSystem = new EventSystem(tree, defaultHitTest);
  });

  // -----------------------------------------------------------------------
  // Mouse events — basic dispatch
  // -----------------------------------------------------------------------

  describe("mouse dispatch", () => {
    it("fires mousedown on target node", () => {
      let received: SyntheticMouseEvent | undefined;
      eventSystem.on(grandchild.nodeId, "mousedown", (ev) => { received = ev; });
      eventSystem.dispatchMouse(mouseOpts());
      expect(received).toBeDefined();
      expect(received!.type).toBe("mousedown");
      expect(received!.target).toBe(grandchild.nodeId);
      expect(received!.button).toBe("left");
    });

    it("fires mouseup on release", () => {
      let received: SyntheticMouseEvent | undefined;
      eventSystem.on(grandchild.nodeId, "mouseup", (ev) => { received = ev; });
      eventSystem.dispatchMouse(mouseOpts({ kind: "release" }));
      expect(received).toBeDefined();
      expect(received!.type).toBe("mouseup");
    });

    it("fires click on release with none button", () => {
      let received: SyntheticMouseEvent | undefined;
      eventSystem.on(grandchild.nodeId, "click", (ev) => { received = ev; });
      eventSystem.dispatchMouse(mouseOpts({ button: "none", kind: "release" }));
      expect(received).toBeDefined();
      expect(received!.type).toBe("click");
    });

    it("fires mousemove on move", () => {
      let received: SyntheticMouseEvent | undefined;
      eventSystem.on(grandchild.nodeId, "mousemove", (ev) => { received = ev; });
      eventSystem.dispatchMouse(mouseOpts({ button: "none", kind: "move" }));
      expect(received).toBeDefined();
      expect(received!.type).toBe("mousemove");
    });
  });

  // -----------------------------------------------------------------------
  // Event bubbling
  // -----------------------------------------------------------------------

  describe("bubbling", () => {
    it("events bubble from target to root", () => {
      const order: number[] = [];
      eventSystem.on(grandchild.nodeId, "mousedown", () => order.push(grandchild.nodeId));
      eventSystem.on(child.nodeId, "mousedown", () => order.push(child.nodeId));
      eventSystem.on(root.nodeId, "mousedown", () => order.push(root.nodeId));

      eventSystem.dispatchMouse(mouseOpts());
      expect(order).toEqual([grandchild.nodeId, child.nodeId, root.nodeId]);
    });

    it("sets currentTarget correctly during bubbling", () => {
      const targets: number[] = [];
      eventSystem.on(grandchild.nodeId, "mousedown", (ev) => targets.push(ev.currentTarget));
      eventSystem.on(child.nodeId, "mousedown", (ev) => targets.push(ev.currentTarget));
      eventSystem.on(root.nodeId, "mousedown", (ev) => targets.push(ev.currentTarget));

      eventSystem.dispatchMouse(mouseOpts());
      expect(targets).toEqual([grandchild.nodeId, child.nodeId, root.nodeId]);
    });

    it("stopPropagation prevents further bubbling", () => {
      const order: number[] = [];
      eventSystem.on(grandchild.nodeId, "mousedown", (ev) => {
        order.push(grandchild.nodeId);
        ev.stopPropagation();
      });
      eventSystem.on(child.nodeId, "mousedown", () => order.push(child.nodeId));
      eventSystem.on(root.nodeId, "mousedown", () => order.push(root.nodeId));

      eventSystem.dispatchMouse(mouseOpts());
      expect(order).toEqual([grandchild.nodeId]);
    });

    it("stopPropagation at intermediate node prevents root from receiving", () => {
      const order: number[] = [];
      eventSystem.on(grandchild.nodeId, "mousedown", () => order.push(grandchild.nodeId));
      eventSystem.on(child.nodeId, "mousedown", (ev) => {
        order.push(child.nodeId);
        ev.stopPropagation();
      });
      eventSystem.on(root.nodeId, "mousedown", () => order.push(root.nodeId));

      eventSystem.dispatchMouse(mouseOpts());
      expect(order).toEqual([grandchild.nodeId, child.nodeId]);
    });
  });

  // -----------------------------------------------------------------------
  // Event delegation
  // -----------------------------------------------------------------------

  describe("delegation", () => {
    it("delegation handlers fire for all events", () => {
      let received: SyntheticMouseEvent | undefined;
      eventSystem.delegate("mousedown", (ev) => { received = ev; });
      eventSystem.dispatchMouse(mouseOpts());
      expect(received).toBeDefined();
      expect(received!.target).toBe(grandchild.nodeId);
    });

    it("delegation dispose works", () => {
      let count = 0;
      const dispose = eventSystem.delegate("mousedown", () => { count++; });
      eventSystem.dispatchMouse(mouseOpts());
      expect(count).toBe(1);
      dispose();
      eventSystem.dispatchMouse(mouseOpts());
      expect(count).toBe(1);
    });
  });

  // -----------------------------------------------------------------------
  // Hover tracking
  // -----------------------------------------------------------------------

  describe("hover tracking", () => {
    it("tracks hovered node on mousemove", () => {
      expect(eventSystem.hoveredNode).toBeUndefined();
      eventSystem.dispatchMouse(mouseOpts({ button: "none", kind: "move" }));
      expect(eventSystem.hoveredNode).toBe(grandchild.nodeId);
    });

    it("fires mouseenter when hovering a new node", () => {
      let entered: SyntheticMouseEvent | undefined;
      eventSystem.on(grandchild.nodeId, "mouseenter", (ev) => { entered = ev; });
      eventSystem.dispatchMouse(mouseOpts({ button: "none", kind: "move" }));
      expect(entered).toBeDefined();
      expect(entered!.type).toBe("mouseenter");
      expect(entered!.target).toBe(grandchild.nodeId);
    });

    it("fires mouseleave when leaving a node", () => {
      // Use a mutable hit target for this test
      let hitTarget: number;
      const mutableHitTest = () => ({
        target: hitTarget,
        path: hitTarget === grandchild.nodeId
          ? [grandchild.nodeId, child.nodeId, root.nodeId]
          : [child.nodeId, root.nodeId],
      });

      resetNodeIdCounter();
      const built = buildTestTree();
      const sys = new EventSystem(built.tree, mutableHitTest);

      let leaveReceived: SyntheticMouseEvent | undefined;
      let enterReceived: SyntheticMouseEvent | undefined;
      sys.on(built.grandchild.nodeId, "mouseleave", (ev) => { leaveReceived = ev; });
      sys.on(built.child.nodeId, "mouseenter", (ev) => { enterReceived = ev; });

      // First move: enter grandchild
      hitTarget = built.grandchild.nodeId;
      sys.dispatchMouse(mouseOpts({ button: "none", kind: "move" }));
      expect(sys.hoveredNode).toBe(built.grandchild.nodeId);

      // Second move: switch to child
      hitTarget = built.child.nodeId;
      sys.dispatchMouse(mouseOpts({ button: "none", kind: "move", col: 6, row: 6 }));
      expect(sys.hoveredNode).toBe(built.child.nodeId);
      expect(leaveReceived).toBeDefined();
      expect(leaveReceived!.type).toBe("mouseleave");
      expect(enterReceived).toBeDefined();
      expect(enterReceived!.type).toBe("mouseenter");
    });

    it("does not fire enter/leave when hovering same node", () => {
      let enterCount = 0;
      eventSystem.on(grandchild.nodeId, "mouseenter", () => { enterCount++; });

      eventSystem.dispatchMouse(mouseOpts({ button: "none", kind: "move" }));
      eventSystem.dispatchMouse(mouseOpts({ button: "none", kind: "move", col: 9, row: 9 }));

      expect(enterCount).toBe(1);
    });

    it("delegation receives mouseenter/mouseleave", () => {
      let enterCount = 0;
      eventSystem.delegate("mouseenter", () => { enterCount++; });
      eventSystem.dispatchMouse(mouseOpts({ button: "none", kind: "move" }));
      expect(enterCount).toBe(1);
    });
  });

  // -----------------------------------------------------------------------
  // Keyboard events
  // -----------------------------------------------------------------------

  describe("keyboard dispatch", () => {
    it("fires keydown on focused node", () => {
      let received: SyntheticKeyboardEvent | undefined;
      eventSystem.on(child.nodeId, "keydown", (ev) => { received = ev; });
      eventSystem.dispatchKeyboard({
        focusedNodeId: child.nodeId,
        key: { type: "char", char: "a" },
        modifiers: NO_MODIFIERS,
        eventType: "press",
      });
      expect(received).toBeDefined();
      expect(received!.type).toBe("keydown");
      expect(received!.key).toEqual({ type: "char", char: "a" });
    });

    it("keyboard events bubble up the tree", () => {
      const order: number[] = [];
      eventSystem.on(grandchild.nodeId, "keydown", () => order.push(grandchild.nodeId));
      eventSystem.on(child.nodeId, "keydown", () => order.push(child.nodeId));
      eventSystem.on(root.nodeId, "keydown", () => order.push(root.nodeId));

      eventSystem.dispatchKeyboard({
        focusedNodeId: grandchild.nodeId,
        key: { type: "enter" },
        modifiers: NO_MODIFIERS,
        eventType: "press",
      });
      expect(order).toEqual([grandchild.nodeId, child.nodeId, root.nodeId]);
    });

    it("stopPropagation works for keyboard events", () => {
      const order: number[] = [];
      eventSystem.on(grandchild.nodeId, "keydown", (ev) => {
        order.push(grandchild.nodeId);
        ev.stopPropagation();
      });
      eventSystem.on(child.nodeId, "keydown", () => order.push(child.nodeId));

      eventSystem.dispatchKeyboard({
        focusedNodeId: grandchild.nodeId,
        key: { type: "enter" },
        modifiers: NO_MODIFIERS,
        eventType: "press",
      });
      expect(order).toEqual([grandchild.nodeId]);
    });

    it("maps repeat to keypress", () => {
      let received: SyntheticKeyboardEvent | undefined;
      eventSystem.on(child.nodeId, "keypress", (ev) => { received = ev; });
      eventSystem.dispatchKeyboard({
        focusedNodeId: child.nodeId,
        key: { type: "char", char: "x" },
        modifiers: NO_MODIFIERS,
        eventType: "repeat",
      });
      expect(received).toBeDefined();
      expect(received!.type).toBe("keypress");
    });

    it("maps release to keyup", () => {
      let received: SyntheticKeyboardEvent | undefined;
      eventSystem.on(child.nodeId, "keyup", (ev) => { received = ev; });
      eventSystem.dispatchKeyboard({
        focusedNodeId: child.nodeId,
        key: { type: "char", char: "x" },
        modifiers: NO_MODIFIERS,
        eventType: "release",
      });
      expect(received).toBeDefined();
      expect(received!.type).toBe("keyup");
    });

    it("delegation receives keyboard events", () => {
      let received: SyntheticKeyboardEvent | undefined;
      eventSystem.delegate("keydown", (ev) => { received = ev; });
      eventSystem.dispatchKeyboard({
        focusedNodeId: child.nodeId,
        key: { type: "tab" },
        modifiers: NO_MODIFIERS,
        eventType: "press",
      });
      expect(received).toBeDefined();
    });
  });

  // -----------------------------------------------------------------------
  // Focus events
  // -----------------------------------------------------------------------

  describe("focus dispatch", () => {
    it("fires focus on target node", () => {
      let received: SyntheticFocusEvent | undefined;
      eventSystem.on(child.nodeId, "focus", (ev) => { received = ev; });
      eventSystem.dispatchFocus("focus", child.nodeId);
      expect(received).toBeDefined();
      expect(received!.type).toBe("focus");
      expect(received!.target).toBe(child.nodeId);
    });

    it("fires blur on target node", () => {
      let received: SyntheticFocusEvent | undefined;
      eventSystem.on(child.nodeId, "blur", (ev) => { received = ev; });
      eventSystem.dispatchFocus("blur", child.nodeId);
      expect(received).toBeDefined();
      expect(received!.type).toBe("blur");
    });

    it("focus events do not bubble", () => {
      let rootReceived = false;
      eventSystem.on(root.nodeId, "focus", () => { rootReceived = true; });
      eventSystem.dispatchFocus("focus", child.nodeId);
      expect(rootReceived).toBe(false);
    });

    it("delegation receives focus events", () => {
      let received: SyntheticFocusEvent | undefined;
      eventSystem.delegate("focus", (ev) => { received = ev; });
      eventSystem.dispatchFocus("focus", child.nodeId);
      expect(received).toBeDefined();
    });
  });

  // -----------------------------------------------------------------------
  // Resize events
  // -----------------------------------------------------------------------

  describe("resize dispatch", () => {
    it("fires resize through delegation", () => {
      let received: SyntheticResizeEvent | undefined;
      eventSystem.delegate("resize", (ev) => { received = ev; });
      eventSystem.dispatchResize({ cols: 80, rows: 24, pixelWidth: 640, pixelHeight: 480 });
      expect(received).toBeDefined();
      expect(received!.cols).toBe(80);
      expect(received!.rows).toBe(24);
    });
  });

  // -----------------------------------------------------------------------
  // Cleanup
  // -----------------------------------------------------------------------

  describe("cleanup", () => {
    it("removeNode clears handlers for a node", () => {
      let count = 0;
      eventSystem.on(grandchild.nodeId, "mousedown", () => { count++; });
      eventSystem.removeNode(grandchild.nodeId);
      eventSystem.dispatchMouse(mouseOpts());
      expect(count).toBe(0);
    });

    it("removeNode clears hover state if node was hovered", () => {
      eventSystem.dispatchMouse(mouseOpts({ button: "none", kind: "move" }));
      expect(eventSystem.hoveredNode).toBe(grandchild.nodeId);
      eventSystem.removeNode(grandchild.nodeId);
      expect(eventSystem.hoveredNode).toBeUndefined();
    });

    it("dispose clears everything", () => {
      let count = 0;
      eventSystem.on(grandchild.nodeId, "mousedown", () => { count++; });
      eventSystem.delegate("mousedown", () => { count++; });
      eventSystem.dispatchMouse(mouseOpts({ button: "none", kind: "move" }));

      eventSystem.dispose();

      eventSystem.dispatchMouse(mouseOpts());
      expect(count).toBe(0);
      expect(eventSystem.hoveredNode).toBeUndefined();
    });
  });

  // -----------------------------------------------------------------------
  // Hit test returns undefined (nothing hit)
  // -----------------------------------------------------------------------

  describe("miss hit test", () => {
    it("handles hit test returning undefined", () => {
      const missSystem = new EventSystem(tree, () => undefined);
      let delegateReceived = false;
      missSystem.delegate("mousedown", () => { delegateReceived = true; });
      missSystem.dispatchMouse(mouseOpts());
      expect(delegateReceived).toBe(true);
    });
  });

  // -----------------------------------------------------------------------
  // off() removes handler
  // -----------------------------------------------------------------------

  describe("off", () => {
    it("removes a specific handler from a node", () => {
      let count = 0;
      const handler = () => { count++; };
      eventSystem.on(grandchild.nodeId, "mousedown", handler);
      eventSystem.off(grandchild.nodeId, "mousedown", handler);
      eventSystem.dispatchMouse(mouseOpts());
      expect(count).toBe(0);
    });

    it("off on non-existent node is a no-op", () => {
      eventSystem.off(999, "mousedown", () => {});
    });
  });
});
