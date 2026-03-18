import type { CSSStyle } from "@kittyui/core";
import type { ReactNode } from "react";

declare module "react/jsx-runtime" {
  namespace JSX {
    interface IntrinsicElements {
      box: { style?: CSSStyle; children?: ReactNode };
      text: { style?: CSSStyle; children?: ReactNode };
    }
  }
}
