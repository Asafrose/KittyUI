# KittyUI

A framework for building rich, graphical terminal applications using the [Kitty graphics protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/).

**Rust core. React frontend. Real apps in your terminal.**

## What is this?

Terminal-based agents are everywhere, but the UIs beside them are still stuck in the text-cell era. KittyUI changes that by combining a high-performance Rust core with React bindings, letting you build real interactive applications — with images, animations, pixel-precise mouse interaction, and a full widget toolkit — all inside your terminal.

## Architecture

```
React JSX (@kittyui/react)
  ↓  Custom React reconciler
TypeScript bindings (@kittyui/core)
  ↓  FFI (napi-rs)
Rust native core
  ├── Layout engine (Taffy — flexbox + grid)
  ├── Cell diffing + ANSI generation
  ├── Kitty graphics protocol engine
  │   ├── Image management (upload, placement, caching)
  │   ├── Animation engine (frame compositing, timing)
  │   ├── Virtual placements (Unicode anchors)
  │   └── Z-index compositor (graphics under/over text)
  ├── Hit-testing + focus management
  └── Event loop (pixel-precise mouse, keyboard, resize)
```

## Key Design Decisions

- **Kitty protocol only** — no Sixel/iTerm2/halfblock fallback. Kitty is winning terminal protocol adoption (Kitty, Ghostty, WezTerm) and we go all-in on its full feature set: images, animations, compositing, z-layering, sub-cell positioning, virtual placements.
- **Rust core** — performance-critical rendering, layout (Taffy), hit-testing, and Kitty protocol encoding all live in Rust. Exposed to JS via napi-rs for Node/Bun compatibility.
- **React bindings** — real React with a custom reconciler (like React Native). Write terminal apps with JSX, hooks, and the component model you already know.
- **Full widget toolkit** — not just `<box>` and `<text>`. Buttons, text inputs, dropdowns, tables, modals, split panes, context menus — everything you need for real applications.
- **First-class mouse support** — pixel-precise coordinates via Kitty's mouse protocol. Hover states, drag interactions, scroll, right-click menus.

## Features (Planned)

### Graphics Primitives
- `<image>` — display images with automatic Kitty protocol negotiation
- `<canvas>` — draw calls that compile to Kitty graphics commands
- `<sprite>` — animated sprites from sprite sheets
- `<layer>` — z-index compositing (graphics beneath or above text)

### Interactive Widgets
- `<button>`, `<checkbox>`, `<radio>`, `<switch>`
- `<textinput>`, `<textarea>`
- `<slider>` — pixel-precise dragging
- `<dropdown>`, `<select>`
- `<scrollview>` — momentum scrolling, draggable scrollbar
- `<tooltip>` — hover-triggered positioning

### Composite Widgets
- `<modal>`, `<dialog>` — focus trapping, backdrop layers
- `<tabs>`, `<accordion>`
- `<table>` — sortable, resizable columns
- `<menu>`, `<contextmenu>`
- `<splitpane>` — draggable dividers
- `<toast>` — timed notifications

## Status

Early development. Stay tuned.

## License

MIT
