import { describe, expect, mock, test, beforeEach } from "bun:test";
import { MutationEncoder, RenderableTree, resetNodeIdCounter } from "@kittyui/core";
import { EventDispatcher } from "./event-dispatcher.js";
import { BoxRenderable } from "./renderables.js";
import type { KittyMouseEvent, KittyKeyboardEvent, KittyFocusEvent } from "./types.js";
import type { KittyEvent } from "@kittyui/core";

// ---------------------------------------------------------------------------
// Mock Bridge
// ---------------------------------------------------------------------------

const createMockBridge = () => ({
  hitTest: mock((_x: number, _y: number): number[] => []),
  getFocusedNode: mock((): number | null => null),
  focus: mock((_id: number) => true),
  blur: mock(() => true),
  onEvents: mock((_listener: (events: KittyEvent[]) => void) => {}),
  pushKeyEvent: mock((_keyCode: number, _modifiers: number, _eventType: number) => {}),
  notifyEventListeners: mock((_events: KittyEvent[]) => {}),
});

type MockBridge = ReturnType<typeof createMockBridge>;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Build a simple tree: root -> child -> grandchild.
 * Returns { tree, root, child, grandchild } with their node IDs.
 */
const buildTree = () => {
  const encoder = new MutationEncoder();
  const tree = new RenderableTree(encoder);

  const root = new BoxRenderable();
  const child = new BoxRenderable();
  const grandchild = new BoxRenderable();

  tree.setRoot(root);
  tree.appendChild(root.nodeId, child);
  tree.appendChild(child.nodeId, grandchild);

  return { tree, root, child, grandchild };
};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("EventDispatcher", () => {
  beforeEach(() => {
    resetNodeIdCounter();
  });

  // -----------------------------------------------------------------------
  // Mouse click dispatching
  // -----------------------------------------------------------------------

  describe("mouse click events", () => {
    test("dispatches onClick to the correct renderable", () => {
      const { tree, child } = buildTree();
      const bridge = createMockBridge();

      // hitTest returns [child.nodeId] (child is the deepest hit)
      bridge.hitTest.mockImplementation(() => [child.nodeId]);

      const dispatcher = new EventDispatcher(bridge as any, tree);
      const onClickSpy = mock((_e: KittyMouseEvent) => {});
      child.eventHandlers.onClick = onClickSpy;

      dispatcher.handleEvents([
        {
          type: "mouse",
          button: 0, // left click
          x: 5,
          y: 5,
          pixelX: 0,
          pixelY: 0,
          modifiers: 0,
          nodeId: child.nodeId,
        },
      ]);

      expect(onClickSpy).toHaveBeenCalledTimes(1);
      const event = onClickSpy.mock.calls[0][0];
      expect(event.x).toBe(5);
      expect(event.y).toBe(5);
    });

    test("event bubbles up to parent when child does not handle it", () => {
      const { tree, root, child, grandchild } = buildTree();
      const bridge = createMockBridge();

      // hitTest returns [grandchild, child, root]
      bridge.hitTest.mockImplementation(() => [
        grandchild.nodeId,
        child.nodeId,
        root.nodeId,
      ]);

      const dispatcher = new EventDispatcher(bridge as any, tree);

      // Only root has an onClick handler — grandchild and child do not
      const rootClickSpy = mock((_e: KittyMouseEvent) => {});
      root.eventHandlers.onClick = rootClickSpy;

      dispatcher.handleEvents([
        {
          type: "mouse",
          button: 0,
          x: 2,
          y: 3,
          pixelX: 0,
          pixelY: 0,
          modifiers: 0,
          nodeId: grandchild.nodeId,
        },
      ]);

      expect(rootClickSpy).toHaveBeenCalledTimes(1);
    });

    test("bubbling stops at the first handler", () => {
      const { tree, root, child, grandchild } = buildTree();
      const bridge = createMockBridge();

      bridge.hitTest.mockImplementation(() => [
        grandchild.nodeId,
        child.nodeId,
        root.nodeId,
      ]);

      const dispatcher = new EventDispatcher(bridge as any, tree);

      const childClickSpy = mock((_e: KittyMouseEvent) => {});
      const rootClickSpy = mock((_e: KittyMouseEvent) => {});
      child.eventHandlers.onClick = childClickSpy;
      root.eventHandlers.onClick = rootClickSpy;

      dispatcher.handleEvents([
        {
          type: "mouse",
          button: 0,
          x: 1,
          y: 1,
          pixelX: 0,
          pixelY: 0,
          modifiers: 0,
          nodeId: grandchild.nodeId,
        },
      ]);

      expect(childClickSpy).toHaveBeenCalledTimes(1);
      expect(rootClickSpy).toHaveBeenCalledTimes(0);
    });
  });

  // -----------------------------------------------------------------------
  // Mouse enter / leave
  // -----------------------------------------------------------------------

  describe("mouse enter/leave events", () => {
    test("fires onMouseEnter when hover target changes", () => {
      const { tree, child, grandchild } = buildTree();
      const bridge = createMockBridge();

      const dispatcher = new EventDispatcher(bridge as any, tree);

      const enterSpy = mock((_e: KittyMouseEvent) => {});
      child.eventHandlers.onMouseEnter = enterSpy;

      // First move: hover enters child
      bridge.hitTest.mockImplementation(() => [child.nodeId]);
      dispatcher.handleEvents([
        {
          type: "mouse",
          button: 35, // move
          x: 3,
          y: 3,
          pixelX: 0,
          pixelY: 0,
          modifiers: 0,
          nodeId: child.nodeId,
        },
      ]);

      expect(enterSpy).toHaveBeenCalledTimes(1);
    });

    test("fires onMouseLeave on old node and onMouseEnter on new node", () => {
      const { tree, child, grandchild } = buildTree();
      const bridge = createMockBridge();

      const dispatcher = new EventDispatcher(bridge as any, tree);

      const childLeaveSpy = mock((_e: KittyMouseEvent) => {});
      const grandchildEnterSpy = mock((_e: KittyMouseEvent) => {});
      child.eventHandlers.onMouseLeave = childLeaveSpy;
      grandchild.eventHandlers.onMouseEnter = grandchildEnterSpy;

      // First: hover over child
      bridge.hitTest.mockImplementation(() => [child.nodeId]);
      dispatcher.handleEvents([
        {
          type: "mouse",
          button: 35,
          x: 3,
          y: 3,
          pixelX: 0,
          pixelY: 0,
          modifiers: 0,
          nodeId: child.nodeId,
        },
      ]);

      // Then: hover moves to grandchild
      bridge.hitTest.mockImplementation(() => [grandchild.nodeId, child.nodeId]);
      dispatcher.handleEvents([
        {
          type: "mouse",
          button: 35,
          x: 4,
          y: 4,
          pixelX: 0,
          pixelY: 0,
          modifiers: 0,
          nodeId: grandchild.nodeId,
        },
      ]);

      expect(childLeaveSpy).toHaveBeenCalledTimes(1);
      expect(grandchildEnterSpy).toHaveBeenCalledTimes(1);
    });

    test("does not fire enter/leave when hover target stays the same", () => {
      const { tree, child } = buildTree();
      const bridge = createMockBridge();

      const dispatcher = new EventDispatcher(bridge as any, tree);

      const enterSpy = mock((_e: KittyMouseEvent) => {});
      child.eventHandlers.onMouseEnter = enterSpy;

      bridge.hitTest.mockImplementation(() => [child.nodeId]);

      // Two moves over the same node
      const mouseMove = {
        type: "mouse" as const,
        button: 35,
        x: 3,
        y: 3,
        pixelX: 0,
        pixelY: 0,
        modifiers: 0,
        nodeId: child.nodeId,
      };

      dispatcher.handleEvents([mouseMove]);
      dispatcher.handleEvents([mouseMove]);

      expect(enterSpy).toHaveBeenCalledTimes(1);
    });
  });

  // -----------------------------------------------------------------------
  // Keyboard events
  // -----------------------------------------------------------------------

  describe("keyboard events", () => {
    test("dispatches onKeyDown to focused node", () => {
      const { tree, child } = buildTree();
      const bridge = createMockBridge();

      bridge.getFocusedNode.mockImplementation(() => child.nodeId);

      const dispatcher = new EventDispatcher(bridge as any, tree);

      const keyDownSpy = mock((_e: KittyKeyboardEvent) => {});
      child.eventHandlers.onKeyDown = keyDownSpy;

      dispatcher.handleEvents([
        {
          type: "keyboard",
          keyCode: 65, // 'A'
          modifiers: 0,
          eventType: 0, // keydown
        },
      ]);

      expect(keyDownSpy).toHaveBeenCalledTimes(1);
    });

    test("keyboard event bubbles up to parent when child does not handle it", () => {
      const { tree, root, child } = buildTree();
      const bridge = createMockBridge();

      bridge.getFocusedNode.mockImplementation(() => child.nodeId);

      const dispatcher = new EventDispatcher(bridge as any, tree);

      // Only root has onKeyDown, child does not
      const rootKeyDownSpy = mock((_e: KittyKeyboardEvent) => {});
      root.eventHandlers.onKeyDown = rootKeyDownSpy;

      dispatcher.handleEvents([
        {
          type: "keyboard",
          keyCode: 66, // 'B'
          modifiers: 0,
          eventType: 0,
        },
      ]);

      expect(rootKeyDownSpy).toHaveBeenCalledTimes(1);
    });

    test("does not dispatch keyboard events when no node is focused", () => {
      const { tree, root } = buildTree();
      const bridge = createMockBridge();

      bridge.getFocusedNode.mockImplementation(() => null);

      const dispatcher = new EventDispatcher(bridge as any, tree);

      const keyDownSpy = mock((_e: KittyKeyboardEvent) => {});
      root.eventHandlers.onKeyDown = keyDownSpy;

      dispatcher.handleEvents([
        {
          type: "keyboard",
          keyCode: 67,
          modifiers: 0,
          eventType: 0,
        },
      ]);

      expect(keyDownSpy).toHaveBeenCalledTimes(0);
    });

    test("handleStdinKeyEvent dispatches to focused node", () => {
      const { tree, child } = buildTree();
      const bridge = createMockBridge();

      bridge.getFocusedNode.mockImplementation(() => child.nodeId);

      const dispatcher = new EventDispatcher(bridge as any, tree);

      const keyDownSpy = mock((_e: KittyKeyboardEvent) => {});
      child.eventHandlers.onKeyDown = keyDownSpy;

      dispatcher.handleStdinKeyEvent(97, 0, 0); // 'a', no modifiers, keydown

      expect(keyDownSpy).toHaveBeenCalledTimes(1);
    });
  });

  // -----------------------------------------------------------------------
  // Focus events
  // -----------------------------------------------------------------------

  describe("focus events", () => {
    test("fires onFocus when a node gains focus", () => {
      const { tree, child } = buildTree();
      const bridge = createMockBridge();

      const dispatcher = new EventDispatcher(bridge as any, tree);

      const focusSpy = mock((_e: KittyFocusEvent) => {});
      child.eventHandlers.onFocus = focusSpy;

      dispatcher.handleEvents([
        { type: "focus", nodeId: child.nodeId },
      ]);

      expect(focusSpy).toHaveBeenCalledTimes(1);
      expect(focusSpy.mock.calls[0][0].nodeId).toBe(child.nodeId);
    });

    test("fires onBlur when a node loses focus", () => {
      const { tree, child } = buildTree();
      const bridge = createMockBridge();

      const dispatcher = new EventDispatcher(bridge as any, tree);

      const blurSpy = mock((_e: KittyFocusEvent) => {});
      child.eventHandlers.onBlur = blurSpy;

      dispatcher.handleEvents([
        { type: "blur", nodeId: child.nodeId },
      ]);

      expect(blurSpy).toHaveBeenCalledTimes(1);
      expect(blurSpy.mock.calls[0][0].nodeId).toBe(child.nodeId);
    });

    test("fires onBlur on old node and onFocus on new node when focus changes", () => {
      const { tree, child, grandchild } = buildTree();
      const bridge = createMockBridge();

      const dispatcher = new EventDispatcher(bridge as any, tree);

      const childBlurSpy = mock((_e: KittyFocusEvent) => {});
      const grandchildFocusSpy = mock((_e: KittyFocusEvent) => {});
      child.eventHandlers.onBlur = childBlurSpy;
      grandchild.eventHandlers.onFocus = grandchildFocusSpy;

      // First: child gains focus
      dispatcher.handleEvents([
        { type: "focus", nodeId: child.nodeId },
      ]);

      // Then: grandchild gains focus (should blur child first)
      dispatcher.handleEvents([
        { type: "focus", nodeId: grandchild.nodeId },
      ]);

      expect(childBlurSpy).toHaveBeenCalledTimes(1);
      expect(grandchildFocusSpy).toHaveBeenCalledTimes(1);
    });
  });

  // -----------------------------------------------------------------------
  // handleMouseFromStdin
  // -----------------------------------------------------------------------

  describe("handleMouseFromStdin", () => {
    test("dispatches onClick for left button press", () => {
      const { tree, child } = buildTree();
      const bridge = createMockBridge();

      bridge.hitTest.mockImplementation(() => [child.nodeId]);

      const dispatcher = new EventDispatcher(bridge as any, tree);
      const onClickSpy = mock((_e: KittyMouseEvent) => {});
      child.eventHandlers.onClick = onClickSpy;

      dispatcher.handleMouseFromStdin(0, 5, 10, false);

      expect(onClickSpy).toHaveBeenCalledTimes(1);
      expect(onClickSpy.mock.calls[0][0].x).toBe(5);
      expect(onClickSpy.mock.calls[0][0].y).toBe(10);
    });

    test("dispatches onMouseUp for release", () => {
      const { tree, child } = buildTree();
      const bridge = createMockBridge();

      bridge.hitTest.mockImplementation(() => [child.nodeId]);

      const dispatcher = new EventDispatcher(bridge as any, tree);
      const mouseUpSpy = mock((_e: KittyMouseEvent) => {});
      child.eventHandlers.onMouseUp = mouseUpSpy;

      dispatcher.handleMouseFromStdin(0, 3, 4, true); // isRelease = true

      expect(mouseUpSpy).toHaveBeenCalledTimes(1);
    });

    test("dispatches onMouseMove for button 35", () => {
      const { tree, child } = buildTree();
      const bridge = createMockBridge();

      bridge.hitTest.mockImplementation(() => [child.nodeId]);

      const dispatcher = new EventDispatcher(bridge as any, tree);
      const mouseMoveSpy = mock((_e: KittyMouseEvent) => {});
      child.eventHandlers.onMouseMove = mouseMoveSpy;

      dispatcher.handleMouseFromStdin(35, 7, 8, false);

      expect(mouseMoveSpy).toHaveBeenCalledTimes(1);
    });

    test("notifies event listeners for useMouse hook", () => {
      const { tree } = buildTree();
      const bridge = createMockBridge();

      bridge.hitTest.mockImplementation(() => []);

      const dispatcher = new EventDispatcher(bridge as any, tree);
      dispatcher.handleMouseFromStdin(0, 1, 1, false);

      expect(bridge.notifyEventListeners).toHaveBeenCalledTimes(1);
    });

    test("dispatches to correct node via hitTest", () => {
      const { tree, root, child, grandchild } = buildTree();
      const bridge = createMockBridge();

      bridge.hitTest.mockImplementation(() => [
        grandchild.nodeId,
        child.nodeId,
        root.nodeId,
      ]);

      const dispatcher = new EventDispatcher(bridge as any, tree);

      const grandchildClickSpy = mock((_e: KittyMouseEvent) => {});
      const rootClickSpy = mock((_e: KittyMouseEvent) => {});
      grandchild.eventHandlers.onClick = grandchildClickSpy;
      root.eventHandlers.onClick = rootClickSpy;

      dispatcher.handleMouseFromStdin(0, 2, 3, false);

      expect(grandchildClickSpy).toHaveBeenCalledTimes(1);
      expect(rootClickSpy).toHaveBeenCalledTimes(0); // bubbling stopped
    });
  });

  // -----------------------------------------------------------------------
  // mouseDown / mouseUp
  // -----------------------------------------------------------------------

  describe("mouseDown / mouseUp events", () => {
    test("dispatches onMouseDown on left button press", () => {
      const { tree, child } = buildTree();
      const bridge = createMockBridge();

      bridge.hitTest.mockImplementation(() => [child.nodeId]);

      const dispatcher = new EventDispatcher(bridge as any, tree);

      const mouseDownSpy = mock((_e: KittyMouseEvent) => {});
      child.eventHandlers.onMouseDown = mouseDownSpy;

      dispatcher.handleEvents([
        {
          type: "mouse",
          button: 0, // left
          x: 1,
          y: 1,
          pixelX: 0,
          pixelY: 0,
          modifiers: 0,
          nodeId: child.nodeId,
        },
      ]);

      expect(mouseDownSpy).toHaveBeenCalledTimes(1);
    });

    test("dispatches onMouseUp on button release", () => {
      const { tree, child } = buildTree();
      const bridge = createMockBridge();

      bridge.hitTest.mockImplementation(() => [child.nodeId]);

      const dispatcher = new EventDispatcher(bridge as any, tree);

      const mouseUpSpy = mock((_e: KittyMouseEvent) => {});
      child.eventHandlers.onMouseUp = mouseUpSpy;

      dispatcher.handleEvents([
        {
          type: "mouse",
          button: 3, // release
          x: 1,
          y: 1,
          pixelX: 0,
          pixelY: 0,
          modifiers: 0,
          nodeId: child.nodeId,
        },
      ]);

      expect(mouseUpSpy).toHaveBeenCalledTimes(1);
    });
  });
});
