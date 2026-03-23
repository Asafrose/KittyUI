// Register KittyUI JSX elements for the react-jsx TypeScript transform.
// This is required because React.JSX.IntrinsicElements needs module
// augmentation that TypeScript can't auto-discover from dependencies.
import type { BoxProps, TextProps, ImageProps } from "@kittyui/react";
declare module "react" { namespace JSX { interface IntrinsicElements { box: BoxProps; text: TextProps; image: ImageProps } } }
