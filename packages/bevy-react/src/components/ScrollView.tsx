import type { ReactNode } from "react";
import type { BevyStyle } from "../types";
import { Node } from "./Intrinsics";

export interface ScrollViewProps {
  children?: ReactNode;
  /** Style for the scroll container (overflow is forced to scroll) */
  style?: BevyStyle;
  /** Scroll axis hint — layout still uses flex; overflow handles clipping/scroll */
  horizontal?: boolean;
}

/**
 * Scrollable container.
 *
 * Sets `overflow: "scroll"` on a host Node so Bevy can clip content.
 * Scrollbar rendering and drag-to-scroll are not implemented yet — the host
 * may scroll via platform/input wiring later; until then this is primarily a
 * clip + style primitive.
 */
export function ScrollView({
  children,
  style,
  horizontal = false,
}: ScrollViewProps): ReactNode {
  const containerStyle: BevyStyle = {
    flexDirection: horizontal ? "row" : "column",
    overflow: "scroll",
    // TODO: Render scrollbar track/thumb UI and wire drag once pointer-move
    // events exist. For now overflow:scroll is the contract with the host.
    ...style,
  };

  return <Node style={containerStyle}>{children}</Node>;
}
