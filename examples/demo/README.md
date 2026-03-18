# KittyUI Demo

A mini dashboard layout demonstrating KittyUI's capabilities:

- Flexbox layouts (header, sidebar, content, footer)
- Nested component hierarchy
- Text styling (bold, colors)
- Background colors
- Gap and padding spacing

## Prerequisites

From the repository root, install dependencies:

```sh
bun install
```

To run with the full terminal renderer, build the native Rust library first:

```sh
bun run build:native
```

## Running

```sh
cd examples/demo
bun run start
```

Without the native library, the demo runs in "tree-only" mode — the React
component tree is mounted and you can verify the node count, but nothing is
rendered to the terminal.

## Type-checking

```sh
bun run typecheck
```
