import type { ReactNode } from "react";
import type { BevyStyle } from "../types";
import { Button, Node, Text } from "./Intrinsics";
import { useInteraction } from "../hooks/useInteraction";

export interface CheckboxProps {
  /** Controlled checked state */
  checked: boolean;
  /** Called when the user toggles the checkbox */
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
 * Toggle checkbox built from Button + Node + Text host primitives.
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
  const { hovered, pressed, handlers } = useInteraction();

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
    borderColor: hovered ? "#8a8aff" : "#6a6a7a",
    backgroundColor: checked
      ? pressed
        ? "#4a4aff"
        : "#5a5aff"
      : pressed
        ? "#3a3a4a"
        : "#2a2a3a",
    ...boxStyle,
  };

  return (
    <Node style={rowStyle}>
      <Button
        {...handlers}
        style={box}
        onClick={() => {
          if (!disabled) {
            onChange(!checked);
          }
        }}
      >
        {checked ? (
          <Text style={{ fontSize: 14, color: "#ffffff" }}>✓</Text>
        ) : null}
      </Button>
      {label != null && label !== "" ? (
        <Text style={{ fontSize: 16, color: "#ffffff", ...labelStyle }}>
          {label}
        </Text>
      ) : null}
    </Node>
  );
}
