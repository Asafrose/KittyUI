/**
 * @kittyui/react — React bindings for KittyUI terminal rendering.
 */

export { hello } from "@kittyui/core";
export { createRoot, type KittyRoot } from "./reconciler.js";
export { BoxRenderable, TextRenderable, createRenderableForType, type KittyProps } from "./renderables.js";
