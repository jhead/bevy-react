import { forwardRef, type ReactNode, type Ref } from "react";
import type { ButtonProps, ImageProps, NodeProps, TextProps } from "../types";
import type { BevyHostInstance } from "../entity";

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
 * Supports `components` (named Rust bundles) and entity refs via `useEntityRef`.
 */
export const Node = forwardRef(function Node(
  props: NodeProps,
  ref: Ref<BevyHostInstance>
): ReactNode {
  // Custom host elements accept reconciler refs; JSX typings omit `ref` on props.
  return <bevy-node {...(props as NodeProps)} {...({ ref } as object)} />;
});

/**
 * Interactive button with click/press events.
 * Maps to Bevy UI `Button` + headless `ui_widgets::Button`.
 */
export const Button = forwardRef(function Button(
  props: ButtonProps,
  ref: Ref<BevyHostInstance>
): ReactNode {
  return <bevy-button {...(props as ButtonProps)} {...({ ref } as object)} />;
});
