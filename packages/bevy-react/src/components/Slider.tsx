import type { ReactNode } from "react";
import type { BevyStyle } from "../types";
import { Node, Text } from "./Intrinsics";

const THUMB_SIZE = 16;
const TRACK_CROSS = 12;
const RAIL_CROSS = 8;

export interface SliderProps {
  /** Current value in [min, max] */
  value: number;
  /** Called when the host slider emits ValueChange */
  onChange: (value: number) => void;
  min?: number;
  max?: number;
  /** Keyboard / track step size (default 1) */
  step?: number;
  /**
   * Layout axis. Bevy 0.17 headless Slider drag/keyboard is horizontal-only;
   * `vertical` still lays out the thumb on Y for visuals.
   */
  orientation?: "horizontal" | "vertical";
  /** Show numeric value label */
  showValue?: boolean;
  disabled?: boolean;
  style?: BevyStyle;
  trackStyle?: BevyStyle;
  fillStyle?: BevyStyle;
  thumbStyle?: BevyStyle;
}

/**
 * Slider mapped to Bevy's headless `ui_widgets::Slider`.
 *
 * Host owns drag / keyboard interaction; React supplies track/fill/thumb look.
 * Thumb travel percent is positioned by the host from `SliderValue`.
 *
 * The `bevy-slider` entity is the track only (not the value label) so Bevy's
 * drag math uses the correct width.
 */
export function Slider({
  value,
  onChange,
  min = 0,
  max = 100,
  step = 1,
  orientation = "horizontal",
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
  const vertical = orientation === "vertical";
  const railInset = (TRACK_CROSS - RAIL_CROSS) / 2;

  const rootStyle: BevyStyle = {
    flexDirection: vertical ? "column" : "row",
    alignItems: "center",
    gap: 8,
    ...(vertical
      ? { minWidth: 28, height: "100%" }
      : { minHeight: 28, width: "100%" }),
    ...style,
  };

  // Slider host entity = track hit target (must not include the value label).
  const track: BevyStyle = {
    ...(vertical
      ? {
          flexGrow: 1,
          flexShrink: 1,
          flexBasis: 0,
          width: TRACK_CROSS,
          height: "100%",
        }
      : {
          flexGrow: 1,
          flexShrink: 1,
          flexBasis: 0,
          height: TRACK_CROSS,
          width: "100%",
        }),
    justifyContent: "center",
    alignItems: "center",
    position: "relative",
    ...trackStyle,
  };

  const rail: BevyStyle = vertical
    ? {
        width: RAIL_CROSS,
        height: "100%",
        backgroundColor: "#2a2a3a",
        borderRadius: 4,
      }
    : {
        height: RAIL_CROSS,
        width: "100%",
        backgroundColor: "#2a2a3a",
        borderRadius: 4,
      };

  const fill: BevyStyle = vertical
    ? {
        position: "absolute",
        left: railInset,
        bottom: 0,
        width: RAIL_CROSS,
        height: fillPercent,
        backgroundColor: "#5a5aff",
        borderRadius: 4,
        pointerEvents: "none",
        ...fillStyle,
      }
    : {
        position: "absolute",
        left: 0,
        top: railInset,
        width: fillPercent,
        height: RAIL_CROSS,
        backgroundColor: "#5a5aff",
        borderRadius: 4,
        pointerEvents: "none",
        ...fillStyle,
      };

  // Inset trailing edge by thumb size so host % travel matches Bevy drag math.
  const travel: BevyStyle = vertical
    ? {
        position: "absolute",
        left: 0,
        right: 0,
        top: 0,
        bottom: THUMB_SIZE,
      }
    : {
        position: "absolute",
        left: 0,
        right: THUMB_SIZE,
        top: 0,
        bottom: 0,
      };

  const thumb: BevyStyle = {
    position: "absolute",
    width: THUMB_SIZE,
    height: THUMB_SIZE,
    ...(vertical
      ? { left: (TRACK_CROSS - THUMB_SIZE) / 2 }
      : { top: (TRACK_CROSS - THUMB_SIZE) / 2 }),
    backgroundColor: "#8a8aff",
    borderRadius: THUMB_SIZE / 2,
    borderWidth: 1,
    borderColor: "#c0c0ff",
    ...thumbStyle,
  };

  const slider = (
    <bevy-slider
      value={clamped}
      min={min}
      max={max}
      step={step}
      orientation={orientation}
      disabled={disabled}
      style={track}
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
      <Node style={rail} />
      <Node style={fill} />
      <Node style={travel}>
        <bevy-slider-thumb style={thumb} />
      </Node>
    </bevy-slider>
  );

  const valueLabel = showValue ? (
    <Text style={{ fontSize: 14, color: "#cccccc", minWidth: 36 }}>
      {String(Math.round(clamped))}
    </Text>
  ) : null;

  return (
    <Node style={rootStyle}>
      {slider}
      {valueLabel}
    </Node>
  );
}
