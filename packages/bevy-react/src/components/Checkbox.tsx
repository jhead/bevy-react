import type { ReactNode } from "react";
import type { BevyStyle } from "../types";
import { Node, Text } from "./Intrinsics";

export interface CheckboxProps {
  /** Controlled checked state */
  checked: boolean;
  /** Called when the host checkbox emits ValueChange */
  onChange: (checked: boolean) => void;
  /** Optional label rendered to the right of the box */
  label?: string;
  /** Disabled — ignores clicks */
  disabled?: boolean;
  style?: BevyStyle;
  boxStyle?: BevyStyle;
  labelStyle?: BevyStyle;
}

/**
 * Toggle checkbox mapped to Bevy's headless `ui_widgets::Checkbox`.
 *
 * Host owns click / keyboard toggle and `style.checked` / `style.hover` visuals
 * (from Bevy `Checked` / picking `Hovered`). React supplies the checkmark child.
 */
export function Checkbox({
  checked,
  onChange,
  label,
  disabled = false,
  style,
  boxStyle,
  labelStyle,
}: CheckboxProps): ReactNode {
  const rowStyle: BevyStyle = {
    flexDirection: "row",
    alignItems: "center",
    gap: 8,
    ...style,
  };

  const box: BevyStyle = {
    width: 20,
    height: 20,
    alignItems: "center",
    justifyContent: "center",
    borderWidth: 2,
    borderColor: "#6a6a7a",
    backgroundColor: "#2a2a3a",
    // Host applies these from Checked / Hovered markers (no React round-trip).
    checked: {
      backgroundColor: "#5a5aff",
      borderColor: "#5a5aff",
    },
    hover: {
      borderColor: "#8a8aff",
    },
    ...boxStyle,
  };

  return (
    <Node style={rowStyle}>
      <bevy-checkbox
        checked={checked}
        disabled={disabled}
        style={box}
        onChange={(event) => {
          if (disabled) return;
          const next =
            event && typeof event === "object" && "value" in event
              ? Boolean((event as { value: boolean }).value)
              : Boolean(event);
          if (next !== checked) {
            onChange(next);
          }
        }}
      >
        {checked ? (
          <Text style={{ fontSize: 14, color: "#ffffff" }}>✓</Text>
        ) : null}
      </bevy-checkbox>
      {label != null && label !== "" ? (
        <Text style={{ fontSize: 16, color: "#ffffff", ...labelStyle }}>
          {label}
        </Text>
      ) : null}
    </Node>
  );
}
