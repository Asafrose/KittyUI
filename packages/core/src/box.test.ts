import { beforeEach, describe, expect, it } from "bun:test";
import {
  type BorderChars,
  BoxRenderable,
  resolveBorderChars,
} from "./box.js";
import type { ComputedLayout } from "./types.js";
import { resetNodeIdCounter } from "./renderable.js";

describe("BoxRenderable", () => {
  beforeEach(() => {
    resetNodeIdCounter();
  });

  // -----------------------------------------------------------------------
  // Construction
  // -----------------------------------------------------------------------

  it("creates an instance with default state", () => {
    const box = new BoxRenderable();
    expect(box.nodeId).toBe(1);
    expect(box.borderChars).toBeUndefined();
    expect(box.borderColor).toBeUndefined();
    expect(box.backgroundColor).toBeUndefined();
    expect(box.overflow).toBe("visible");
    expect(box.shadow).toBeUndefined();
  });

  it("static create() applies style", () => {
    const box = BoxRenderable.create({
      background: "red",
      border: "single",
      height: 10,
      width: 20,
    });
    expect(box.borderChars).toBeDefined();
    expect(box.backgroundColor).toEqual({ b: 0, g: 0, r: 255, type: "rgb" });
    expect(box.nodeStyle.width).toEqual({ type: "cells", value: 20 });
    expect(box.nodeStyle.height).toEqual({ type: "cells", value: 10 });
  });

  it("static create() with no style returns clean box", () => {
    const box = BoxRenderable.create();
    expect(box.borderChars).toBeUndefined();
  });

  // -----------------------------------------------------------------------
  // Borders
  // -----------------------------------------------------------------------

  describe("borders", () => {
    it("sets single border preset", () => {
      const box = BoxRenderable.create({ border: "single" });
      const chars = box.borderChars!;
      expect(chars.topLeft).toBe("\u250C");
      expect(chars.horizontal).toBe("\u2500");
      expect(chars.vertical).toBe("\u2502");
    });

    it("sets double border preset", () => {
      const box = BoxRenderable.create({ border: "double" });
      const chars = box.borderChars!;
      expect(chars.topLeft).toBe("\u2554");
      expect(chars.horizontal).toBe("\u2550");
    });

    it("sets rounded border preset", () => {
      const box = BoxRenderable.create({ border: "rounded" });
      const chars = box.borderChars!;
      expect(chars.topLeft).toBe("\u256D");
      expect(chars.bottomRight).toBe("\u256F");
    });

    it("sets bold border preset", () => {
      const box = BoxRenderable.create({ border: "bold" });
      const chars = box.borderChars!;
      expect(chars.topLeft).toBe("\u250F");
      expect(chars.horizontal).toBe("\u2501");
    });

    it("accepts custom border characters", () => {
      const custom: BorderChars = {
        bottomLeft: "+",
        bottomRight: "+",
        horizontal: "-",
        topLeft: "+",
        topRight: "+",
        vertical: "|",
      };
      const box = BoxRenderable.create({ border: custom });
      expect(box.borderChars).toEqual(custom);
    });

    it("removes border with false", () => {
      const box = BoxRenderable.create({ border: "single" });
      expect(box.borderChars).toBeDefined();
      box.setBorder(false);
      expect(box.borderChars).toBeUndefined();
    });

    it("sets border color", () => {
      const box = BoxRenderable.create({
        border: "single",
        borderColor: "#ff0000",
      });
      expect(box.borderColor).toEqual({ b: 0, g: 0, r: 255, type: "rgb" });
    });

    it("sets border color with Color object", () => {
      const box = new BoxRenderable();
      box.setBorderColor({ index: 3, type: "ansi" });
      expect(box.borderColor).toEqual({ index: 3, type: "ansi" });
    });

    it("clears border color with undefined", () => {
      const box = BoxRenderable.create({ borderColor: "red" });
      box.setBorderColor(undefined);
      expect(box.borderColor).toBeUndefined();
    });

    it("renders border cells for a 4x3 box", () => {
      const box = BoxRenderable.create({ border: "single" });
      const layout: ComputedLayout = { height: 3, width: 4, x: 0, y: 0 };
      const cells = box.renderBorder(layout);

      // 4 corners + 2 horizontal top + 2 horizontal bottom + 1 left + 1 right = 10
      expect(cells.length).toBe(10);

      // Corners
      expect(cells).toContainEqual({ char: "\u250C", col: 0, row: 0 });
      expect(cells).toContainEqual({ char: "\u2510", col: 3, row: 0 });
      expect(cells).toContainEqual({ char: "\u2514", col: 0, row: 2 });
      expect(cells).toContainEqual({ char: "\u2518", col: 3, row: 2 });
    });

    it("returns empty border cells for too-small box", () => {
      const box = BoxRenderable.create({ border: "single" });
      const cells = box.renderBorder({ height: 1, width: 1, x: 0, y: 0 });
      expect(cells).toEqual([]);
    });

    it("returns empty border cells when no border set", () => {
      const box = new BoxRenderable();
      const cells = box.renderBorder({ height: 5, width: 10, x: 0, y: 0 });
      expect(cells).toEqual([]);
    });

    it("returns empty border cells when no layout", () => {
      const box = BoxRenderable.create({ border: "single" });
      const cells = box.renderBorder();
      expect(cells).toEqual([]);
    });
  });

  // -----------------------------------------------------------------------
  // resolveBorderChars
  // -----------------------------------------------------------------------

  describe("resolveBorderChars", () => {
    it("resolves all presets", () => {
      const presets = ["single", "double", "rounded", "bold"] as const;
      for (const preset of presets) {
        const chars = resolveBorderChars(preset);
        expect(chars.topLeft).toBeDefined();
        expect(chars.horizontal).toBeDefined();
        expect(chars.vertical).toBeDefined();
      }
    });
  });

  // -----------------------------------------------------------------------
  // Background
  // -----------------------------------------------------------------------

  describe("background", () => {
    it("sets background color via style", () => {
      const box = BoxRenderable.create({ background: "blue" });
      expect(box.backgroundColor).toEqual({ b: 255, g: 0, r: 0, type: "rgb" });
    });

    it("sets background color via backgroundColor", () => {
      const box = BoxRenderable.create({ backgroundColor: "#00ff00" });
      expect(box.backgroundColor).toEqual({ b: 0, g: 255, r: 0, type: "rgb" });
    });

    it("setBackgroundColor updates the color", () => {
      const box = new BoxRenderable();
      box.setBackgroundColor("red");
      expect(box.backgroundColor).toEqual({ b: 0, g: 0, r: 255, type: "rgb" });
    });

    it("clears background with undefined", () => {
      const box = BoxRenderable.create({ background: "red" });
      box.setBackgroundColor(undefined);
      expect(box.backgroundColor).toBeUndefined();
    });

    it("renders background cells inside border", () => {
      const box = BoxRenderable.create({ background: "red", border: "single" });
      const layout: ComputedLayout = { height: 3, width: 4, x: 0, y: 0 };
      const cells = box.renderBackground(layout);
      // Inside a 4x3 box with border: 2x1 interior
      expect(cells.length).toBe(2);
      expect(cells).toContainEqual({ col: 1, row: 1 });
      expect(cells).toContainEqual({ col: 2, row: 1 });
    });

    it("renders background cells without border", () => {
      const box = BoxRenderable.create({ background: "red" });
      const layout: ComputedLayout = { height: 2, width: 3, x: 0, y: 0 };
      const cells = box.renderBackground(layout);
      // Full 3x2 = 6 cells
      expect(cells.length).toBe(6);
    });

    it("returns empty when no background color", () => {
      const box = new BoxRenderable();
      const cells = box.renderBackground({ height: 5, width: 5, x: 0, y: 0 });
      expect(cells).toEqual([]);
    });

    it("returns empty when no layout", () => {
      const box = BoxRenderable.create({ background: "red" });
      expect(box.renderBackground()).toEqual([]);
    });
  });

  // -----------------------------------------------------------------------
  // Overflow
  // -----------------------------------------------------------------------

  describe("overflow", () => {
    it("defaults to visible", () => {
      const box = new BoxRenderable();
      expect(box.overflow).toBe("visible");
    });

    it("sets overflow via style", () => {
      const box = BoxRenderable.create({ overflow: "hidden" });
      expect(box.overflow).toBe("hidden");
    });

    it("sets overflow via setOverflow", () => {
      const box = new BoxRenderable();
      box.setOverflow("scroll");
      expect(box.overflow).toBe("scroll");
      expect(box.dirty).toBe(true);
    });
  });

  // -----------------------------------------------------------------------
  // Box shadow
  // -----------------------------------------------------------------------

  describe("box shadow", () => {
    it("sets shadow via style", () => {
      const box = BoxRenderable.create({
        boxShadow: { char: "\u2588", color: "gray", offsetX: 2, offsetY: 1 },
      });
      expect(box.shadow).toBeDefined();
      expect(box.shadow!.offsetX).toBe(2);
      expect(box.shadow!.offsetY).toBe(1);
      expect(box.shadow!.char).toBe("\u2588");
      expect(box.shadow!.color).toEqual({ b: 128, g: 128, r: 128, type: "rgb" });
    });

    it("applies default shadow values", () => {
      const box = BoxRenderable.create({ boxShadow: {} });
      expect(box.shadow!.offsetX).toBe(1);
      expect(box.shadow!.offsetY).toBe(1);
      expect(box.shadow!.char).toBe("\u2591");
      expect(box.shadow!.color).toBeUndefined();
    });

    it("removes shadow with undefined", () => {
      const box = BoxRenderable.create({ boxShadow: { offsetX: 1 } });
      expect(box.shadow).toBeDefined();
      box.setBoxShadow(undefined);
      expect(box.shadow).toBeUndefined();
    });

    it("renders shadow cells for bottom and right", () => {
      const box = new BoxRenderable();
      box.setBoxShadow({ offsetX: 1, offsetY: 1 });
      const layout: ComputedLayout = { height: 2, width: 3, x: 0, y: 0 };
      const cells = box.renderShadow(layout);

      // Bottom row: 3 cells (cols 1..3 at row 2)
      // Right col: 1 cell (col 3 at row 1)
      expect(cells.length).toBe(4);
      expect(cells).toContainEqual({ char: "\u2591", col: 1, row: 2 });
      expect(cells).toContainEqual({ char: "\u2591", col: 2, row: 2 });
      expect(cells).toContainEqual({ char: "\u2591", col: 3, row: 2 });
      expect(cells).toContainEqual({ char: "\u2591", col: 3, row: 1 });
    });

    it("returns empty shadow when offsets are zero", () => {
      const box = new BoxRenderable();
      box.setBoxShadow({ offsetX: 0, offsetY: 0 });
      const cells = box.renderShadow({ height: 5, width: 5, x: 0, y: 0 });
      expect(cells).toEqual([]);
    });

    it("returns empty shadow when no shadow set", () => {
      const box = new BoxRenderable();
      const cells = box.renderShadow({ height: 5, width: 5, x: 0, y: 0 });
      expect(cells).toEqual([]);
    });

    it("returns empty shadow when no layout", () => {
      const box = new BoxRenderable();
      box.setBoxShadow({ offsetX: 1, offsetY: 1 });
      expect(box.renderShadow()).toEqual([]);
    });
  });

  // -----------------------------------------------------------------------
  // Padding and margin via CSS style
  // -----------------------------------------------------------------------

  describe("padding and margin", () => {
    it("applies padding from style", () => {
      const box = BoxRenderable.create({ padding: 2 });
      const pad = box.nodeStyle.padding;
      expect(pad).toBeDefined();
      expect(pad![0]).toEqual({ type: "cells", value: 2 });
      expect(pad![3]).toEqual({ type: "cells", value: 2 });
    });

    it("applies margin from style", () => {
      const box = BoxRenderable.create({ margin: [1, 2] });
      const margin = box.nodeStyle.margin;
      expect(margin).toBeDefined();
      expect(margin![0]).toEqual({ type: "cells", value: 1 });
      expect(margin![1]).toEqual({ type: "cells", value: 2 });
    });

    it("border ensures minimum 1-cell padding", () => {
      const box = BoxRenderable.create({ border: "single", padding: 0 });
      const pad = box.nodeStyle.padding;
      expect(pad).toBeDefined();
      // Border adds at least 1 cell padding on each side
      expect(pad![0]).toEqual({ type: "cells", value: 1 });
    });

    it("border preserves larger padding", () => {
      const box = BoxRenderable.create({ border: "single", padding: 3 });
      const pad = box.nodeStyle.padding;
      expect(pad![0]).toEqual({ type: "cells", value: 3 });
    });
  });

  // -----------------------------------------------------------------------
  // Flexbox / grid properties
  // -----------------------------------------------------------------------

  describe("flexbox and grid properties", () => {
    it("maps flex properties to node style", () => {
      const box = BoxRenderable.create({
        alignItems: "stretch",
        display: "flex",
        flexDirection: "column",
        flexGrow: 1,
        flexShrink: 0,
        gap: 2,
        justifyContent: "center",
      });
      const display = box.nodeStyle.display;
      expect(display).toBeDefined();
      expect(display!.type).toBe("flex");
      if (display!.type === "flex") {
        expect(display!.flex!.direction).toBe("column");
        expect(display!.flex!.justify).toBe("center");
        expect(display!.flex!.alignItems).toBe("stretch");
        expect(display!.flex!.grow).toBe(1);
        expect(display!.flex!.shrink).toBe(0);
      }
      expect(box.nodeStyle.gap).toEqual([
        { type: "cells", value: 2 },
        { type: "cells", value: 2 },
      ]);
    });

    it("maps grid properties to node style", () => {
      const box = BoxRenderable.create({
        columnGap: 1,
        display: "grid",
        gridTemplateColumns: [10, "1fr", "auto"],
        gridTemplateRows: [5, "2fr"],
        rowGap: 1,
      });
      const display = box.nodeStyle.display;
      expect(display).toBeDefined();
      expect(display!.type).toBe("grid");
      if (display!.type === "grid") {
        expect(display!.grid!.columns).toHaveLength(3);
        expect(display!.grid!.rows).toHaveLength(2);
        expect(display!.grid!.columnGap).toEqual({ type: "cells", value: 1 });
      }
    });
  });

  // -----------------------------------------------------------------------
  // Content area
  // -----------------------------------------------------------------------

  describe("getContentArea", () => {
    it("returns full layout when no border or padding", () => {
      const box = new BoxRenderable();
      const layout: ComputedLayout = { height: 10, width: 20, x: 5, y: 3 };
      box.updateLayout(layout);
      const content = box.getContentArea();
      expect(content).toEqual({ height: 10, width: 20, x: 5, y: 3 });
    });

    it("accounts for border inset", () => {
      const box = BoxRenderable.create({ border: "single" });
      const layout: ComputedLayout = { height: 6, width: 10, x: 0, y: 0 };
      const content = box.getContentArea(layout);
      // Border inset (1) + minimum border padding (1) = 2 on each side
      expect(content).toEqual({ height: 2, width: 6, x: 2, y: 2 });
    });

    it("accounts for explicit padding", () => {
      const box = BoxRenderable.create({ padding: 2 });
      const layout: ComputedLayout = { height: 10, width: 20, x: 0, y: 0 };
      const content = box.getContentArea(layout);
      expect(content).toEqual({ height: 6, width: 16, x: 2, y: 2 });
    });

    it("accounts for border + larger padding", () => {
      const box = BoxRenderable.create({ border: "single", padding: 3 });
      const layout: ComputedLayout = { height: 10, width: 20, x: 0, y: 0 };
      const content = box.getContentArea(layout);
      // border inset (1) + padding (3) = 4 on each side
      expect(content).toEqual({ height: 2, width: 12, x: 4, y: 4 });
    });

    it("clamps content area to zero", () => {
      const box = BoxRenderable.create({ border: "single", padding: 10 });
      const layout: ComputedLayout = { height: 5, width: 5, x: 0, y: 0 };
      const content = box.getContentArea(layout);
      expect(content!.width).toBe(0);
      expect(content!.height).toBe(0);
    });

    it("returns undefined when no layout", () => {
      const box = new BoxRenderable();
      expect(box.getContentArea()).toBeUndefined();
    });
  });

  // -----------------------------------------------------------------------
  // Dirty tracking
  // -----------------------------------------------------------------------

  describe("dirty tracking", () => {
    it("marks dirty on setBoxStyle", () => {
      const box = new BoxRenderable();
      box.clearDirty();
      box.setBoxStyle({ width: 10 });
      expect(box.dirty).toBe(true);
    });

    it("marks dirty on setBorder", () => {
      const box = new BoxRenderable();
      box.clearDirty();
      box.setBorder("double");
      expect(box.dirty).toBe(true);
    });

    it("marks dirty on setBorderColor", () => {
      const box = new BoxRenderable();
      box.clearDirty();
      box.setBorderColor("red");
      expect(box.dirty).toBe(true);
    });

    it("marks dirty on setBackgroundColor", () => {
      const box = new BoxRenderable();
      box.clearDirty();
      box.setBackgroundColor("blue");
      expect(box.dirty).toBe(true);
    });

    it("marks dirty on setOverflow", () => {
      const box = new BoxRenderable();
      box.clearDirty();
      box.setOverflow("hidden");
      expect(box.dirty).toBe(true);
    });

    it("marks dirty on setBoxShadow", () => {
      const box = new BoxRenderable();
      box.clearDirty();
      box.setBoxShadow({ offsetX: 1, offsetY: 1 });
      expect(box.dirty).toBe(true);
    });
  });

  // -----------------------------------------------------------------------
  // Lifecycle (inherited from Renderable)
  // -----------------------------------------------------------------------

  describe("lifecycle", () => {
    it("extends Renderable", () => {
      const box = new BoxRenderable();
      expect(box).toBeInstanceOf(BoxRenderable);
      // Should have Renderable methods
      expect(typeof box.setStyle).toBe("function");
      expect(typeof box.onMount).toBe("function");
      expect(typeof box.onUnmount).toBe("function");
    });
  });
});
