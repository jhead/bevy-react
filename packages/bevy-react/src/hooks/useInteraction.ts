import { useState, useCallback } from "react";
import type { ButtonProps } from "../types";

export interface InteractionState {
  /** True while the pointer is over the target */
  hovered: boolean;
  /**
   * True while pressed.
   * Requires `onPress` / `onRelease` events from the host — currently may stay
   * false until press/release dispatch is wired end-to-end.
   */
  pressed: boolean;
  /** Spread onto `<Button>` (or any host that accepts these props) */
  handlers: Pick<
    ButtonProps,
    "onHover" | "onMouseEnter" | "onMouseLeave" | "onPress" | "onRelease"
  >;
}

/**
 * Tracks hover/pressed visual state for interactive host components.
 *
 * Hover uses `onMouseEnter` / `onMouseLeave` (and `onHover`).
 * Pressed uses `onPress` / `onRelease` when the host emits them.
 */
export function useInteraction(): InteractionState {
  const [hovered, setHovered] = useState(false);
  const [pressed, setPressed] = useState(false);

  const onMouseEnter = useCallback(() => {
    setHovered(true);
  }, []);

  const onHover = useCallback(() => {
    setHovered(true);
  }, []);

  const onMouseLeave = useCallback(() => {
    setHovered(false);
    setPressed(false);
  }, []);

  const onPress = useCallback(() => {
    setPressed(true);
  }, []);

  const onRelease = useCallback(() => {
    setPressed(false);
  }, []);

  return {
    hovered,
    pressed,
    handlers: {
      onHover,
      onMouseEnter,
      onMouseLeave,
      onPress,
      onRelease,
    },
  };
}
