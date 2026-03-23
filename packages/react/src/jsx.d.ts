/**
 * JSX intrinsic element declarations for KittyUI.
 *
 * Augments both the global JSX namespace (for classic JSX transform) and
 * the React.JSX namespace (for react-jsx transform) so that <box>, <text>,
 * and <image> elements are type-checked in TSX files automatically when
 * @kittyui/react is imported.
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

declare module "react" {
  // eslint-disable-next-line @typescript-eslint/no-namespace
  namespace JSX {
    interface IntrinsicElements {
      box: BoxProps;
      text: TextProps;
      image: ImageProps;
    }
  }
}
