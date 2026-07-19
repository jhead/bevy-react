import type { ReactNode } from "react";
import type { BevyStyle } from "../types";
import { Button, Node, Text } from "./Intrinsics";

export interface SliderProps {
  /** Current value in [min, max] */
  value: number;
  /** Called when value changes */
  onChange: (value: number) => void;
  min?: number;
  max?: number;
  /** Step size for +/- controls (default 1) */
  step?: number;
  /** Show numeric value label */
  showValue?: boolean;
  disabled?: boolean;
  style?: BevyStyle;
  trackStyle?: BevyStyle;
  fillStyle?: BevyStyle;
}

/**
 * Horizontal slider built from Node/Button primitives.
 *
 * Full track drag is not available yet (no pointer-move events). Value is
 * adjusted via step buttons; the track fill reflects the current ratio.
 */
export function Slider({
  value,
  onChange,
  min = 0,
  max = 100,
  step = 1,
  showValue = false,
  disabled = false,
  style,
  trackStyle,
  fillStyle,
}: SliderProps): ReactNode {
  const clamped = Math.min(max, Math.max(min, value));
  const range = max - min;
  const ratio = range <= 0 ? 0 : (clamped - min) / range;
  const fillPercent = `${Math.round(ratio * 100)}%`;

  const rowStyle: BevyStyle = {
    flexDirection: "row",
    alignItems: "center",
    gap: 8,
    minHeight: 28,
    ...style,
  };

  const track: BevyStyle = {
    flexGrow: 1,
    flexShrink: 1,
    flexBasis: 0,
    height: 8,
    backgroundColor: "#2a2a3a",
    borderRadius: 4,
    ...trackStyle,
  };

  const fill: BevyStyle = {
    width: fillPercent,
    height: "100%",
    backgroundColor: "#5a5aff",
    borderRadius: 4,
    ...fillStyle,
  };

  const stepButtonStyle: BevyStyle = {
    width: 28,
    height: 28,
    alignItems: "center",
    justifyContent: "center",
    backgroundColor: "#3a3a4a",
    borderWidth: 1,
    borderColor: "#5a5a6a",
  };

  const adjust = (delta: number) => {
    if (disabled) return;
    const next = Math.min(max, Math.max(min, clamped + delta));
    if (next !== clamped) {
      onChange(next);
    }
  };

  return (
    <Node style={rowStyle}>
      <Button style={stepButtonStyle} onClick={() => adjust(-step)}>
        <Text style={{ fontSize: 14, color: "#ffffff" }}>-</Text>
      </Button>
      {/* Track — visual only until pointer-drag is wired */}
      <Node style={track}>
        <Node style={fill} />
      </Node>
      <Button style={stepButtonStyle} onClick={() => adjust(step)}>
        <Text style={{ fontSize: 14, color: "#ffffff" }}>+</Text>
      </Button>
      {showValue ? (
        <Text style={{ fontSize: 14, color: "#cccccc", minWidth: 36 }}>
          {String(Math.round(clamped))}
        </Text>
      ) : null}
    </Node>
  );
}
