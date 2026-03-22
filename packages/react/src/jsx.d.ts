/**
 * JSX intrinsic element declarations for KittyUI.
 *
 * Augments the global JSX namespace so that <box>, <text>, and <image>
 * elements are type-checked in TSX files.
 */

import type { BoxProps, ImageProps, TextProps } from "./types.js";

declare global {
  // eslint-disable-next-line @typescript-eslint/no-namespace
  namespace JSX {
    interface IntrinsicElements {
      box: BoxProps;
      text: TextProps;
      image: ImageProps;
    }
  }
}
