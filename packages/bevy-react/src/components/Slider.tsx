import type { ReactNode } from "react";
import type { BevyStyle } from "../types";
import { Node, Text } from "./Intrinsics";

export interface SliderProps {
  /** Current value in [min, max] */
  value: number;
  /** Called when the host slider emits ValueChange */
  onChange: (value: number) => void;
  min?: number;
  max?: number;
  /** Keyboard / track step size (default 1) */
  step?: number;
  /** Show numeric value label */
  showValue?: boolean;
  disabled?: boolean;
  style?: BevyStyle;
  trackStyle?: BevyStyle;
  fillStyle?: BevyStyle;
  thumbStyle?: BevyStyle;
}

/**
 * Horizontal slider mapped to Bevy's headless `ui_widgets::Slider`.
 *
 * Host owns drag / keyboard interaction; React supplies track/fill/thumb look.
 * Thumb `left` is positioned by the host from `SliderValue`.
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
  thumbStyle,
}: SliderProps): ReactNode {
  const clamped = Math.min(max, Math.max(min, value));
  const range = max - min;
  const ratio = range <= 0 ? 0 : (clamped - min) / range;
  const fillPercent = `${Math.round(ratio * 100)}%`;

  const rootStyle: BevyStyle = {
    flexDirection: "row",
    alignItems: "center",
    gap: 8,
    minHeight: 28,
    width: "100%",
    ...style,
  };

  const track: BevyStyle = {
    flexGrow: 1,
    flexShrink: 1,
    flexBasis: 0,
    height: 12,
    justifyContent: "center",
    position: "relative",
    ...trackStyle,
  };

  const rail: BevyStyle = {
    height: 8,
    width: "100%",
    backgroundColor: "#2a2a3a",
    borderRadius: 4,
  };

  const fill: BevyStyle = {
    position: "absolute",
    left: 0,
    top: 2,
    width: fillPercent,
    height: 8,
    backgroundColor: "#5a5aff",
    borderRadius: 4,
    pointerEvents: "none",
    ...fillStyle,
  };

  // Travel track is inset by thumb width so host % positioning stays in sync.
  const travel: BevyStyle = {
    position: "absolute",
    left: 0,
    right: 16,
    top: 0,
    bottom: 0,
  };

  const thumb: BevyStyle = {
    position: "absolute",
    width: 16,
    height: 16,
    top: -2,
    backgroundColor: "#8a8aff",
    borderRadius: 8,
    borderWidth: 1,
    borderColor: "#c0c0ff",
    ...thumbStyle,
  };

  return (
    <bevy-slider
      value={clamped}
      min={min}
      max={max}
      step={step}
      disabled={disabled}
      style={rootStyle}
      onChange={(event) => {
        if (disabled) return;
        const next =
          event && typeof event === "object" && "value" in event
            ? Number((event as { value: number }).value)
            : Number(event);
        if (!Number.isNaN(next) && next !== clamped) {
          onChange(next);
        }
      }}
    >
      <Node style={track}>
        <Node style={rail} />
        <Node style={fill} />
        <Node style={travel}>
          <bevy-slider-thumb style={thumb} />
        </Node>
      </Node>
      {showValue ? (
        <Text style={{ fontSize: 14, color: "#cccccc", minWidth: 36 }}>
          {String(Math.round(clamped))}
        </Text>
      ) : null}
    </bevy-slider>
  );
}
