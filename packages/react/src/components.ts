/**
 * KittyUI React components — Box, Text, Image.
 *
 * These are thin wrappers around the intrinsic elements ("box", "text", "image")
 * that the reconciler knows how to create. Using uppercase components means
 * consumers don't need JSX intrinsic element type augmentation.
 */

import { createElement, forwardRef, type ReactNode } from "react";
import type { BoxProps, TextProps, ImageProps } from "./types.js";
import type { BoxRenderable, TextRenderable } from "./renderables.js";

/** Container element with flexbox layout, background colors, and event handling. */
export const Box = forwardRef<BoxRenderable, BoxProps>(function Box(props, ref) {
  return createElement("box", { ...props, ref });
});

/** Text element for rendering string content with colors and styles. */
export const Text = forwardRef<TextRenderable, TextProps>(function Text(props, ref) {
  return createElement("text", { ...props, ref });
});

/** Image element for displaying images via the Kitty graphics protocol. */
export const Image = forwardRef<BoxRenderable, ImageProps>(function Image(props, ref) {
  return createElement("image", { ...props, ref });
});
