import type { ReactNode } from "react";
import type { BevyStyle } from "../types";
import { Node } from "./Intrinsics";

export interface PortalProps {
  children?: ReactNode;
  /**
   * Optional wrapper style. Defaults to an absolute full-bleed overlay so
   * children can stack above siblings via zIndex.
   */
  style?: BevyStyle;
  /** Stacking order hint (maps to BevyStyle.zIndex when supported) */
  zIndex?: number;
}

/**
 * Overlay / portal-like primitive for tooltips and modals.
 *
 * Limitation: `preparePortalMount` in the reconciler is a stub, and this
 * package does not yet expose `reconciler.createPortal` with a separate
 * container (no GlobalZIndex / UiTargetCamera remount). Children render in
 * the same React tree as the caller. Use absolute positioning + `zIndex`
 * for overlay stacking until a real portal mount exists.
 */
export function Portal({
  children,
  style,
  zIndex = 1000,
}: PortalProps): ReactNode {
  const overlayStyle: BevyStyle = {
    position: "absolute",
    left: 0,
    top: 0,
    right: 0,
    bottom: 0,
    zIndex,
    ...style,
  };

  return <Node style={overlayStyle}>{children}</Node>;
}
