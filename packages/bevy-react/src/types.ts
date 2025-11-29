import type { ReactNode } from "react";

/**
 * Style properties that map to Bevy's UI Style component.
 * Values can be:
 * - Numbers (treated as pixels)
 * - Strings like "100px", "50%", "auto"
 */
export interface BevyStyle {
  // Size
  width?: string | number;
  height?: string | number;
  minWidth?: string | number;
  minHeight?: string | number;
  maxWidth?: string | number;
  maxHeight?: string | number;

  // Position
  position?: "relative" | "absolute";
  left?: string | number;
  right?: string | number;
  top?: string | number;
  bottom?: string | number;

  // Flexbox
  flexDirection?: "row" | "column" | "rowReverse" | "columnReverse";
  flexWrap?: "noWrap" | "wrap" | "wrapReverse";
  flexGrow?: number;
  flexShrink?: number;
  flexBasis?: string | number;
  alignItems?: "start" | "end" | "center" | "baseline" | "stretch";
  alignSelf?: "auto" | "start" | "end" | "center" | "baseline" | "stretch";
  alignContent?:
    | "start"
    | "end"
    | "center"
    | "stretch"
    | "spaceBetween"
    | "spaceAround"
    | "spaceEvenly";
  justifyContent?:
    | "start"
    | "end"
    | "center"
    | "spaceBetween"
    | "spaceAround"
    | "spaceEvenly";
  justifyItems?: "start" | "end" | "center" | "baseline" | "stretch";
  justifySelf?: "auto" | "start" | "end" | "center" | "baseline" | "stretch";

  // Spacing
  margin?: string | number;
  marginLeft?: string | number;
  marginRight?: string | number;
  marginTop?: string | number;
  marginBottom?: string | number;
  padding?: string | number;
  paddingLeft?: string | number;
  paddingRight?: string | number;
  paddingTop?: string | number;
  paddingBottom?: string | number;

  // Border
  borderWidth?: string | number;
  borderColor?: string;
  borderRadius?: string | number;

  // Visual
  backgroundColor?: string;

  // Display
  display?: "flex" | "none";
  overflow?: "visible" | "clip" | "scroll";
}

/**
 * Props for the <node> element (NodeBundle)
 */
export interface NodeProps {
  style?: BevyStyle;
  children?: ReactNode;
}

/**
 * Props for the <button> element (ButtonBundle with interaction)
 */
export interface ButtonProps extends NodeProps {
  onClick?: () => void;
  onPress?: () => void;
  onRelease?: () => void;
  onHover?: () => void;
}

/**
 * Keyboard event data
 */
export interface KeyboardEventData {
  key: string;
}

/**
 * Props for the internal <bevy-text-input> element (focusable container)
 */
export interface TextInputInternalProps extends NodeProps {
  onFocus?: () => void;
  onBlur?: () => void;
  onKeyDown?: (event: KeyboardEventData) => void;
}

/**
 * Props for the <text> element (TextBundle)
 */
export interface TextProps {
  children?: ReactNode;
  style?: BevyStyle & {
    fontSize?: number;
    color?: string;
    fontFamily?: string;
  };
}

/**
 * Props for the <image> element (ImageBundle)
 */
export interface ImageProps {
  src: string;
  style?: BevyStyle;
}

/**
 * Internal instance type used by the reconciler
 */
export interface BevyInstance {
  nodeId: number;
  type: string;
  props: Record<string, unknown>;
  children: BevyInstance[];
}

/**
 * Text instance type
 */
export interface BevyTextInstance {
  nodeId: number;
  text: string;
}

/**
 * Internal JSX augmentation for host elements.
 * Users should import the exported components (Node, Button, Text, Image) instead.
 */
type BevyIntrinsicElements = {
  "bevy-node": NodeProps;
  "bevy-button": ButtonProps;
  "bevy-text": TextProps;
  "bevy-image": ImageProps;
  "bevy-text-input": TextInputInternalProps;
};

declare global {
  namespace JSX {
    interface IntrinsicElements extends BevyIntrinsicElements {}
  }
}

declare module "react" {
  namespace JSX {
    interface IntrinsicElements extends BevyIntrinsicElements {}
  }
}
