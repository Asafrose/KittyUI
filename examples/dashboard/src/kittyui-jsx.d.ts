/**
 * KittyUI JSX intrinsic element declarations for the dashboard example.
 * Augments both global JSX and React.JSX to cover all TS configurations.
 */

import type { BoxProps, ImageProps, TextProps } from "@kittyui/react";
import "react";

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
