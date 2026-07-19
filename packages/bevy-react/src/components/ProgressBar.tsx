import type { ReactNode } from "react";
import type { BevyStyle } from "../types";
import { Node, Text } from "./Intrinsics";

export interface ProgressBarProps {
  /** Progress in [0, 1], or use `value`/`max` instead */
  progress?: number;
  /** Absolute value (used with `max`) */
  value?: number;
  /** Maximum for `value` (default 1) */
  max?: number;
  /** Show percentage label */
  showLabel?: boolean;
  style?: BevyStyle;
  trackStyle?: BevyStyle;
  fillStyle?: BevyStyle;
  labelStyle?: BevyStyle;
}

/**
 * Deterministic progress bar from Node + Text host primitives.
 */
export function ProgressBar({
  progress,
  value,
  max = 1,
  showLabel = false,
  style,
  trackStyle,
  fillStyle,
  labelStyle,
}: ProgressBarProps): ReactNode {
  let ratio: number;
  if (progress != null) {
    ratio = progress;
  } else if (value != null) {
    ratio = max <= 0 ? 0 : value / max;
  } else {
    ratio = 0;
  }
  ratio = Math.min(1, Math.max(0, ratio));
  const percent = Math.round(ratio * 100);

  const outer: BevyStyle = {
    flexDirection: "column",
    gap: 4,
    ...style,
  };

  const track: BevyStyle = {
    width: "100%",
    height: 12,
    backgroundColor: "#2a2a3a",
    borderRadius: 4,
    overflow: "clip",
    ...trackStyle,
  };

  const fill: BevyStyle = {
    width: `${percent}%`,
    height: "100%",
    backgroundColor: "#5a5aff",
    borderRadius: 4,
    ...fillStyle,
  };

  return (
    <Node style={outer}>
      <Node style={track}>
        <Node style={fill} />
      </Node>
      {showLabel ? (
        <Text style={{ fontSize: 12, color: "#cccccc", ...labelStyle }}>
          {`${percent}%`}
        </Text>
      ) : null}
    </Node>
  );
}
