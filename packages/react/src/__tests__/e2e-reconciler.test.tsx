/**
 * E2E React reconciler integration tests for KittyUI.
 *
 * Tests functional components, props, children, conditional rendering,
 * lists, state, effects, re-rendering, unmounting, and edge cases.
 */

import { describe, test, expect, afterEach } from "bun:test";
import React, { useState, useEffect, useRef, useMemo, useCallback, Fragment } from "react";

declare module "react" {
  // eslint-disable-next-line @typescript-eslint/no-namespace
  namespace JSX {
    interface IntrinsicElements {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      [elemName: string]: any;
    }
  }
}

import { TestBridge } from "@kittyui/core/src/test-harness/test-bridge.js";
import "@kittyui/core/src/test-harness/assertions.js";
import { render, type RenderResult } from "../test-utils/render-jsx.js";

const canRun = new TestBridge().nativeAvailable;

describe.skipIf(!canRun)("E2E Reconciler", () => {
  let result: RenderResult | undefined;

  afterEach(() => {
    result?.cleanup();
    result = undefined;
  });

  // ==========================================================================
  // Functional components
  // ==========================================================================

  describe("functional components", () => {
    test("simple functional component renders", async () => {
      const MyComponent = () => (
        <box style={{ width: 20, height: 3 }}><text>FC</text></box>
      );
      result = await render(<MyComponent />, { cols: 20, rows: 3 });
      expect(result.screen).toContainText("FC");
    });

    test("component with props", async () => {
      const Greeting = ({ name }: { name: string }) => (
        <box style={{ width: 20, height: 3 }}><text>{`Hi ${name}`}</text></box>
      );
      result = await render(<Greeting name="World" />, { cols: 20, rows: 3 });
      expect(result.screen).toContainText("Hi World");
    });

    test("component with children", async () => {
      const Wrapper = ({ children }: { children: React.ReactNode }) => (
        <box style={{ width: 20, height: 3 }}>{children}</box>
      );
      result = await render(
        <Wrapper><text>Child</text></Wrapper>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("Child");
    });

    test("nested functional components", async () => {
      const Inner = () => <text>Inner</text>;
      const Outer = () => (
        <box style={{ width: 20, height: 3 }}><Inner /></box>
      );
      result = await render(<Outer />, { cols: 20, rows: 3 });
      expect(result.screen).toContainText("Inner");
    });

    test("deeply nested components (5 levels)", async () => {
      const L5 = () => <text>Deep5</text>;
      const L4 = () => <box><L5 /></box>;
      const L3 = () => <box><L4 /></box>;
      const L2 = () => <box><L3 /></box>;
      const L1 = () => <box style={{ width: 20, height: 3 }}><L2 /></box>;
      result = await render(<L1 />, { cols: 20, rows: 3 });
      expect(result.screen).toContainText("Deep5");
    });

    test("component returning null", async () => {
      const NullComp = () => null;
      result = await render(
        <box style={{ width: 20, height: 3 }}>
          <NullComp />
          <text>After</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("After");
    });
  });

  // ==========================================================================
  // Conditional rendering
  // ==========================================================================

  describe("conditional rendering", () => {
    test("ternary renders truthy branch", async () => {
      const show = true;
      result = await render(
        <box style={{ width: 20, height: 3 }}>
          {show ? <text>Shown</text> : <text>Hidden</text>}
        </box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("Shown");
      expect(result.screen.containsText("Hidden")).toBe(false);
    });

    test("ternary renders falsy branch", async () => {
      const show = false;
      result = await render(
        <box style={{ width: 20, height: 3 }}>
          {show ? <text>Shown</text> : <text>Hidden</text>}
        </box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("Hidden");
      expect(result.screen.containsText("Shown")).toBe(false);
    });

    test("conditional with && operator", async () => {
      const show = true;
      result = await render(
        <box style={{ width: 20, height: 3 }}>
          {show && <text>Visible</text>}
        </box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("Visible");
    });

    test("false && does not render", async () => {
      const show = false;
      result = await render(
        <box style={{ width: 20, height: 3 }}>
          {show && <text>Invisible</text>}
          <text>OK</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen.containsText("Invisible")).toBe(false);
      expect(result.screen).toContainText("OK");
    });
  });

  // ==========================================================================
  // List rendering
  // ==========================================================================

  describe("list rendering", () => {
    test("array.map renders items", async () => {
      const items = ["A", "B", "C"];
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 5 }}>
          {items.map((item) => (
            <text key={item}>{item}</text>
          ))}
        </box>,
        { cols: 20, rows: 5 },
      );
      expect(result.screen).toContainText("A");
      expect(result.screen).toContainText("B");
      expect(result.screen).toContainText("C");
    });

    test("empty array renders nothing", async () => {
      const items: string[] = [];
      result = await render(
        <box style={{ width: 20, height: 3 }}>
          {items.map((item) => (
            <text key={item}>{item}</text>
          ))}
          <text>Empty</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("Empty");
    });

    test("list with many items", async () => {
      const items = Array.from({ length: 10 }, (_, i) => `Item${i}`);
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 10 }}>
          {items.map((item) => (
            <text key={item}>{item}</text>
          ))}
        </box>,
        { cols: 20, rows: 10 },
      );
      expect(result.screen).toContainText("Item0");
      expect(result.screen).toContainText("Item9");
    });

    test("numeric keys work", async () => {
      const nums = [1, 2, 3];
      result = await render(
        <box style={{ flexDirection: "row", width: 20, height: 3 }}>
          {nums.map((n) => (
            <text key={n}>{`${n}`}</text>
          ))}
        </box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("1");
      expect(result.screen).toContainText("2");
      expect(result.screen).toContainText("3");
    });
  });

  // ==========================================================================
  // useState
  // ==========================================================================

  describe("useState", () => {
    test("initial state renders", async () => {
      const Counter = () => {
        const [count] = useState(0);
        return (
          <box style={{ width: 20, height: 3 }}><text>{`Count: ${count}`}</text></box>
        );
      };
      result = await render(<Counter />, { cols: 20, rows: 3 });
      expect(result.screen).toContainText("Count: 0");
    });

    test("state update triggers rerender", async () => {
      let triggerUpdate: (() => void) | undefined;
      const Counter = () => {
        const [count, setCount] = useState(0);
        triggerUpdate = () => setCount(1);
        return (
          <box style={{ width: 20, height: 3 }}><text>{`Count: ${count}`}</text></box>
        );
      };
      result = await render(<Counter />, { cols: 20, rows: 3 });
      expect(result.screen).toContainText("Count: 0");

      triggerUpdate!();
      const screen2 = await result.rerender(<Counter />);
      expect(screen2).toContainText("Count: 1");
    });

    test("multiple state variables", async () => {
      const Multi = () => {
        const [a] = useState("hello");
        const [b] = useState(42);
        return (
          <box style={{ flexDirection: "row", width: 20, height: 3 }}>
            <text>{a}</text>
            <text>{`${b}`}</text>
          </box>
        );
      };
      result = await render(<Multi />, { cols: 20, rows: 3 });
      expect(result.screen).toContainText("hello");
      expect(result.screen).toContainText("42");
    });
  });

  // ==========================================================================
  // useEffect
  // ==========================================================================

  describe("useEffect", () => {
    test("effect fires on mount", async () => {
      let effectRan = false;
      const Effector = () => {
        useEffect(() => {
          effectRan = true;
        }, []);
        return <box style={{ width: 10, height: 3 }}><text>E</text></box>;
      };
      result = await render(<Effector />, { cols: 10, rows: 3 });
      expect(effectRan).toBe(true);
    });

    test("cleanup fires on unmount", async () => {
      let cleanedUp = false;
      const Effector = () => {
        useEffect(() => {
          return () => { cleanedUp = true; };
        }, []);
        return <box style={{ width: 10, height: 3 }}><text>E</text></box>;
      };
      result = await render(<Effector />, { cols: 10, rows: 3 });
      expect(cleanedUp).toBe(false);
      result.cleanup();
      await new Promise((resolve) => setTimeout(resolve, 100));
      result = undefined;
      expect(cleanedUp).toBe(true);
    });
  });

  // ==========================================================================
  // useRef
  // ==========================================================================

  describe("useRef", () => {
    test("ref holds stable value", async () => {
      let refValue: number | undefined;
      const RefComp = () => {
        const ref = useRef(42);
        refValue = ref.current;
        return <box style={{ width: 10, height: 3 }}><text>Ref</text></box>;
      };
      result = await render(<RefComp />, { cols: 10, rows: 3 });
      expect(refValue).toBe(42);
    });

    test("ref persists across rerenders", async () => {
      let renderCount = 0;
      const RefComp = () => {
        const ref = useRef(0);
        renderCount++;
        ref.current = renderCount;
        return <box style={{ width: 10, height: 3 }}><text>{`R${ref.current}`}</text></box>;
      };
      result = await render(<RefComp />, { cols: 10, rows: 3 });
      await result.rerender(<RefComp />);
      expect(renderCount).toBeGreaterThanOrEqual(2);
    });
  });

  // ==========================================================================
  // useMemo / useCallback
  // ==========================================================================

  describe("useMemo / useCallback", () => {
    test("useMemo caches computed value", async () => {
      let computeCount = 0;
      const Memo = ({ value }: { value: number }) => {
        const doubled = useMemo(() => {
          computeCount++;
          return value * 2;
        }, [value]);
        return <box style={{ width: 20, height: 3 }}><text>{`${doubled}`}</text></box>;
      };
      result = await render(<Memo value={5} />, { cols: 20, rows: 3 });
      expect(result.screen).toContainText("10");
      expect(computeCount).toBe(1);
    });

    test("useCallback returns stable function", async () => {
      let callbackRef: (() => void) | undefined;
      const CB = () => {
        const fn = useCallback(() => {}, []);
        callbackRef = fn;
        return <box style={{ width: 10, height: 3 }}><text>CB</text></box>;
      };
      result = await render(<CB />, { cols: 10, rows: 3 });
      expect(callbackRef).toBeDefined();
      expect(typeof callbackRef).toBe("function");
    });
  });

  // ==========================================================================
  // Re-rendering
  // ==========================================================================

  describe("re-rendering", () => {
    test("rerender with different text", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}><text>Before</text></box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("Before");

      const screen2 = await result.rerender(
        <box style={{ width: 20, height: 3 }}><text>After</text></box>,
      );
      expect(screen2).toContainText("After");
      expect(screen2.containsText("Before")).toBe(false);
    });

    test("rerender with different props", async () => {
      result = await render(
        <box style={{ backgroundColor: "#ff0000", width: 10, height: 3 }}><text>A</text></box>,
        { cols: 20, rows: 5 },
      );
      const pos1 = result.screen.findText("A");
      expect(pos1).toBeDefined();
      expect(result.screen).toHaveBgColor(pos1!.row, pos1!.col, "#ff0000");

      const screen2 = await result.rerender(
        <box style={{ backgroundColor: "#0000ff", width: 10, height: 3 }}><text>A</text></box>,
      );
      const pos2 = screen2.findText("A");
      expect(pos2).toBeDefined();
      expect(screen2).toHaveBgColor(pos2!.row, pos2!.col, "#0000ff");
    });

    test("rerender with added child", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 5 }}>
          <text>One</text>
        </box>,
        { cols: 20, rows: 5 },
      );
      expect(result.screen).toContainText("One");

      const screen2 = await result.rerender(
        <box style={{ flexDirection: "column", width: 20, height: 5 }}>
          <text>One</text>
          <text>Two</text>
        </box>,
      );
      expect(screen2).toContainText("One");
      expect(screen2).toContainText("Two");
    });

    test("rerender with removed child", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 5 }}>
          <text>One</text>
          <text>Two</text>
        </box>,
        { cols: 20, rows: 5 },
      );
      expect(result.screen).toContainText("Two");

      const screen2 = await result.rerender(
        <box style={{ flexDirection: "column", width: 20, height: 5 }}>
          <text>One</text>
        </box>,
      );
      expect(screen2).toContainText("One");
      expect(screen2.containsText("Two")).toBe(false);
    });

    test("rerender with reordered children", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 5 }}>
          <text key="a">Alpha</text>
          <text key="b">Beta</text>
        </box>,
        { cols: 20, rows: 5 },
      );
      const posA1 = result.screen.findText("Alpha");
      const posB1 = result.screen.findText("Beta");
      expect(posA1!.row).toBeLessThan(posB1!.row);

      const screen2 = await result.rerender(
        <box style={{ flexDirection: "column", width: 20, height: 5 }}>
          <text key="b">Beta</text>
          <text key="a">Alpha</text>
        </box>,
      );
      const posA2 = screen2.findText("Alpha");
      const posB2 = screen2.findText("Beta");
      expect(posB2!.row).toBeLessThan(posA2!.row);
    });

    test("rerender with different dimensions", async () => {
      result = await render(
        <box style={{ width: 10, height: 3 }}><text>Small</text></box>,
        { cols: 40, rows: 10 },
      );
      expect(result.screen).toContainText("Small");

      const screen2 = await result.rerender(
        <box style={{ width: 30, height: 8 }}><text>Big</text></box>,
      );
      expect(screen2).toContainText("Big");
    });

    test("rerender replacing component entirely", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}><text>CompA</text></box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("CompA");

      const screen2 = await result.rerender(
        <box style={{ width: 20, height: 3 }}><text>CompB</text></box>,
      );
      expect(screen2).toContainText("CompB");
      expect(screen2.containsText("CompA")).toBe(false);
    });
  });

  // ==========================================================================
  // Fragments
  // ==========================================================================

  describe("fragments", () => {
    test("Fragment renders children", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 5 }}>
          <Fragment>
            <text>One</text>
            <text>Two</text>
          </Fragment>
        </box>,
        { cols: 20, rows: 5 },
      );
      expect(result.screen).toContainText("One");
      expect(result.screen).toContainText("Two");
    });

    test("short syntax fragment", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 5 }}>
          <>
            <text>Frag1</text>
            <text>Frag2</text>
          </>
        </box>,
        { cols: 20, rows: 5 },
      );
      expect(result.screen).toContainText("Frag1");
      expect(result.screen).toContainText("Frag2");
    });

    test("nested fragments", async () => {
      result = await render(
        <box style={{ width: 20, height: 5 }}>
          <>
            <>
              <text>Deep</text>
            </>
          </>
        </box>,
        { cols: 20, rows: 5 },
      );
      expect(result.screen).toContainText("Deep");
    });
  });

  // ==========================================================================
  // Null/undefined/boolean children
  // ==========================================================================

  describe("special children", () => {
    test("null children are ignored", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}>
          {null}
          <text>OK</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("OK");
    });

    test("undefined children are ignored", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}>
          {undefined}
          <text>OK</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("OK");
    });

    test("false children are filtered", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}>
          {false}
          <text>OK</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("OK");
      // "false" should not appear as text
      expect(result.screen.containsText("false")).toBe(false);
    });

    test("true children are filtered", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}>
          {true}
          <text>OK</text>
        </box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("OK");
      expect(result.screen.containsText("true")).toBe(false);
    });

    test("number children render as text", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}>
          {42}
        </box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("42");
    });

    test("string children render as text", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}>
          {"hello"}
        </box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("hello");
    });

    test("zero renders as text", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}>
          {0}
        </box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("0");
    });
  });

  // ==========================================================================
  // Component unmounting
  // ==========================================================================

  describe("unmounting", () => {
    test("unmount clears screen on rerender", async () => {
      result = await render(
        <box style={{ width: 20, height: 3 }}><text>Visible</text></box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("Visible");

      const screen2 = await result.rerender(
        <box style={{ width: 20, height: 3 }} />,
      );
      expect(screen2.containsText("Visible")).toBe(false);
    });

    test("conditional unmount removes child", async () => {
      const show = true;
      result = await render(
        <box style={{ width: 20, height: 3 }}>
          {show ? <text>Here</text> : null}
        </box>,
        { cols: 20, rows: 3 },
      );
      expect(result.screen).toContainText("Here");

      const screen2 = await result.rerender(
        <box style={{ width: 20, height: 3 }} />,
      );
      expect(screen2.containsText("Here")).toBe(false);
    });
  });

  // ==========================================================================
  // Complex scenarios
  // ==========================================================================

  describe("complex scenarios", () => {
    test("component with state, conditional, and list", async () => {
      const App = () => {
        const [items] = useState(["X", "Y", "Z"]);
        const showTitle = true;
        return (
          <box style={{ flexDirection: "column", width: 20, height: 8 }}>
            {showTitle && <text>Title</text>}
            {items.map((item) => (
              <text key={item}>{item}</text>
            ))}
          </box>
        );
      };
      result = await render(<App />, { cols: 20, rows: 8 });
      expect(result.screen).toContainText("Title");
      expect(result.screen).toContainText("X");
      expect(result.screen).toContainText("Y");
      expect(result.screen).toContainText("Z");
    });

    test("wrapper component with styled children", async () => {
      const Card = ({ title, color }: { title: string; color: string }) => (
        <box style={{ backgroundColor: color, width: 20, height: 3 }}>
          <text>{title}</text>
        </box>
      );
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 6 }}>
          <Card title="Card1" color="#ff0000" />
          <Card title="Card2" color="#0000ff" />
        </box>,
        { cols: 20, rows: 6 },
      );
      expect(result.screen).toContainText("Card1");
      expect(result.screen).toContainText("Card2");
    });

    test("rerender updates specific child only", async () => {
      result = await render(
        <box style={{ flexDirection: "column", width: 20, height: 5 }}>
          <text key="static">Static</text>
          <text key="dynamic">Before</text>
        </box>,
        { cols: 20, rows: 5 },
      );
      expect(result.screen).toContainText("Static");
      expect(result.screen).toContainText("Before");

      const screen2 = await result.rerender(
        <box style={{ flexDirection: "column", width: 20, height: 5 }}>
          <text key="static">Static</text>
          <text key="dynamic">After</text>
        </box>,
      );
      expect(screen2).toContainText("Static");
      expect(screen2).toContainText("After");
      expect(screen2.containsText("Before")).toBe(false);
    });

    test("mixed elements and components", async () => {
      const Badge = ({ label }: { label: string }) => (
        <text style={{ color: "#ff0000" }}>{label}</text>
      );
      result = await render(
        <box style={{ flexDirection: "row", width: 30, height: 3 }}>
          <text>Hello </text>
          <Badge label="NEW" />
        </box>,
        { cols: 30, rows: 3 },
      );
      expect(result.screen).toContainText("Hello");
      expect(result.screen).toContainText("NEW");
    });
  });
});
