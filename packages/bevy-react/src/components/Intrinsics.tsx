import { type ReactNode } from "react";
import type { ButtonProps, ImageProps, NodeProps, TextProps } from "../types";

/**
 * Text display component. Maps to Bevy's Text component.
 */
export function Text(props: TextProps): ReactNode {
  return <bevy-text {...props}>{props.children}</bevy-text>;
}

/**
 * Image display component. Maps to Bevy's ImageNode.
 */
export function Image(props: ImageProps): ReactNode {
  return <bevy-image {...props} />;
}

/**
 * Container node for layout. Maps to Bevy's Node component.
 */
export function Node({ children, style }: NodeProps): ReactNode {
  return <bevy-node style={style}>{children}</bevy-node>;
}

/**
 * Interactive button with click/press events.
 */
export function Button(props: ButtonProps): ReactNode {
  return <bevy-button {...props}>{props.children}</bevy-button>;
}
