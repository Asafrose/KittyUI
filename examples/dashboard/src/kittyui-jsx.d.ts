/**
 * JSX intrinsic element declarations for KittyUI.
 *
 * The react-jsx TypeScript transform uses React.JSX (not global JSX),
 * so we augment the "react" module to add box, text, and image elements.
 */
import type { CSSStyle } from "@kittyui/core";

declare module "react" {
  namespace JSX {
    interface IntrinsicElements {
      box: { style?: CSSStyle; key?: string | number; children?: React.ReactNode };
      text: { style?: CSSStyle; key?: string | number; children?: React.ReactNode };
      image: { style?: CSSStyle; src?: string; key?: string | number };
    }
  }
}
