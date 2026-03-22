/**
 * Tests for JSX intrinsic types, props, and event handlers.
 *
 * These tests verify both compile-time type correctness and runtime prop handling.
 */

import { describe, expect, test } from "bun:test";
import { resetNodeIdCounter } from "@kittyui/core";
import {
  BoxRenderable,
  ImageRenderable,
  TextRenderable,
  createRenderableForType,
} from "./renderables.js";
import type {
  BoxProps,
  ImageProps,
  KittyFocusEvent,
  KittyKeyboardEvent,
  KittyMouseEvent,
  KittyScrollEvent,
  TextProps,
} from "./types.js";

describe("JSX intrinsic types", () => {
  // Reset IDs before each test for determinism
  test.each([undefined])("setup", () => {
    resetNodeIdCounter();
  });

  // -------------------------------------------------------------------------
  // Compile-time type checks (these fail to compile if types are wrong)
  // -------------------------------------------------------------------------

  test("BoxProps accepts style and event handlers", () => {
    const props: BoxProps = {
      style: { width: 10, height: 5, flexDirection: "row", backgroundColor: "#ff0000" },
      onClick: (_e: KittyMouseEvent) => {},
      onKeyDown: (_e: KittyKeyboardEvent) => {},
      onFocus: (_e: KittyFocusEvent) => {},
      tabIndex: 0,
      autoFocus: true,
    };
    expect(props.style?.width).toBe(10);
    expect(props.tabIndex).toBe(0);
    expect(props.autoFocus).toBe(true);
    expect(typeof props.onClick).toBe("function");
    expect(typeof props.onKeyDown).toBe("function");
    expect(typeof props.onFocus).toBe("function");
  });

  test("TextProps accepts style and string children", () => {
    const props: TextProps = {
      style: { color: "red", fontWeight: "bold", fontStyle: "italic", textDecoration: "underline" },
      children: "Hello, KittyUI!",
    };
    expect(props.children).toBe("Hello, KittyUI!");
    expect(props.style?.fontWeight).toBe("bold");
  });

  test("ImageProps requires src", () => {
    const props: ImageProps = {
      src: "/path/to/image.png",
      width: 40,
      height: 20,
      style: { margin: 1 },
    };
    expect(props.src).toBe("/path/to/image.png");
    expect(props.width).toBe(40);
    expect(props.height).toBe(20);
  });

  test("BoxProps accepts all mouse event handlers", () => {
    const handlers: BoxProps = {
      onClick: () => {},
      onMouseEnter: () => {},
      onMouseLeave: () => {},
      onMouseDown: () => {},
      onMouseUp: () => {},
      onMouseMove: () => {},
      onScroll: (_e: KittyScrollEvent) => {},
    };
    expect(typeof handlers.onClick).toBe("function");
    expect(typeof handlers.onScroll).toBe("function");
  });

  test("BoxProps accepts all keyboard event handlers", () => {
    const handlers: BoxProps = {
      onKeyDown: () => {},
      onKeyUp: () => {},
      onKeyPress: () => {},
    };
    expect(typeof handlers.onKeyDown).toBe("function");
    expect(typeof handlers.onKeyUp).toBe("function");
    expect(typeof handlers.onKeyPress).toBe("function");
  });

  test("BoxProps accepts focus/blur handlers", () => {
    const handlers: BoxProps = {
      onFocus: () => {},
      onBlur: () => {},
    };
    expect(typeof handlers.onFocus).toBe("function");
    expect(typeof handlers.onBlur).toBe("function");
  });

  // -------------------------------------------------------------------------
  // Runtime prop handling
  // -------------------------------------------------------------------------

  test("BoxRenderable.applyProps stores event handlers", () => {
    const box = new BoxRenderable();
    const onClick = (): void => {};
    const onKeyDown = (): void => {};

    box.applyProps({
      style: { width: 20 },
      onClick,
      onKeyDown,
      tabIndex: 1,
      autoFocus: true,
    });

    expect(box.eventHandlers.onClick).toBe(onClick);
    expect(box.eventHandlers.onKeyDown).toBe(onKeyDown);
    expect(box.tabIndex).toBe(1);
    expect(box.autoFocus).toBe(true);
  });

  test("BoxRenderable.applyProps clears removed handlers", () => {
    const box = new BoxRenderable();
    const onClick = (): void => {};

    box.applyProps({ onClick });
    expect(box.eventHandlers.onClick).toBe(onClick);

    box.applyProps({});
    expect(box.eventHandlers.onClick).toBeUndefined();
  });

  test("TextRenderable.applyProps applies style", () => {
    const text = new TextRenderable();
    text.applyProps({ style: { color: "#00ff00" } });
    // The style was applied — textStyle should have fg set
    expect(text.textStyle.fg).toEqual({ type: "rgb", r: 0, g: 255, b: 0 });
  });

  test("ImageRenderable.applyProps stores src and dimensions", () => {
    const img = new ImageRenderable();
    img.applyProps({
      src: "/path/to/cat.png",
      width: 80,
      height: 24,
      style: { margin: 2 },
    });

    expect(img.src).toBe("/path/to/cat.png");
    expect(img.displayWidth).toBe(80);
    expect(img.displayHeight).toBe(24);
    // Style was applied
    expect(img.nodeStyle.margin).toBeDefined();
  });

  test("createRenderableForType creates ImageRenderable", () => {
    const instance = createRenderableForType("image");
    expect(instance).toBeInstanceOf(ImageRenderable);
    expect(instance.type).toBe("image");
  });

  test("createRenderableForType throws for unknown type", () => {
    expect(() => createRenderableForType("canvas")).toThrow('Unknown KittyUI element type: "canvas"');
  });

  // -------------------------------------------------------------------------
  // Style type coverage
  // -------------------------------------------------------------------------

  test("CSSStyle supports full flexbox properties via BoxProps", () => {
    const props: BoxProps = {
      style: {
        width: 100,
        height: 50,
        minWidth: 10,
        minHeight: 5,
        maxWidth: 200,
        maxHeight: 100,
        flexDirection: "column",
        flexGrow: 1,
        flexShrink: 0,
        flexBasis: "auto",
        flexWrap: "wrap",
        justifyContent: "space-between",
        alignItems: "center",
        padding: [1, 2, 3, 4],
        margin: 1,
        gap: [1, 2],
        backgroundColor: "#333",
        color: "#fff",
        fontWeight: "bold",
        fontStyle: "italic",
        textDecoration: "underline",
      },
    };
    expect(props.style?.flexDirection).toBe("column");
    expect(props.style?.justifyContent).toBe("space-between");
  });

  test("Ref types are accepted on BoxProps", () => {
    let captured: BoxRenderable | null = null;
    const props: BoxProps = {
      ref: (instance) => {
        captured = instance;
      },
    };
    // Call the ref callback to verify the type works
    if (typeof props.ref === "function") {
      props.ref(new BoxRenderable());
    }
    expect(captured).toBeInstanceOf(BoxRenderable);
  });

  test("Ref object type is accepted on BoxProps", () => {
    const refObj: { current: BoxRenderable | null } = { current: null };
    const props: BoxProps = { ref: refObj };
    expect(props.ref).toBe(refObj);
  });
});
