import { useState, type ReactNode } from "react";
import type { BevyStyle } from "../types";
import { Button, Node, Text } from "./Intrinsics";
import { useInteraction } from "../hooks/useInteraction";

export interface SelectOption<T extends string = string> {
  value: T;
  label: string;
}

export interface SelectProps<T extends string = string> {
  value: T;
  options: SelectOption<T>[];
  onChange: (value: T) => void;
  disabled?: boolean;
  placeholder?: string;
  style?: BevyStyle;
  menuStyle?: BevyStyle;
  optionStyle?: BevyStyle;
}

/**
 * Dropdown select built from Button + Node + Text.
 *
 * The menu is an in-tree overlay (absolute under the trigger), not a true
 * portal — see Portal for stacking limitations.
 */
export function Select<T extends string = string>({
  value,
  options,
  onChange,
  disabled = false,
  placeholder = "Select…",
  style,
  menuStyle,
  optionStyle,
}: SelectProps<T>): ReactNode {
  const [open, setOpen] = useState(false);
  const { hovered, handlers } = useInteraction();

  const selected = options.find((o) => o.value === value);
  const display = selected?.label ?? placeholder;

  const triggerStyle: BevyStyle = {
    flexDirection: "row",
    alignItems: "center",
    justifyContent: "spaceBetween",
    padding: 8,
    minWidth: 140,
    minHeight: 36,
    backgroundColor: hovered ? "#3a3a4a" : "#2a2a3a",
    borderWidth: 2,
    borderColor: open ? "#6a6aff" : "#4a4a5a",
    ...style,
  };

  const menu: BevyStyle = {
    position: "absolute",
    top: "100%",
    left: 0,
    right: 0,
    flexDirection: "column",
    backgroundColor: "#2a2a3a",
    borderWidth: 1,
    borderColor: "#4a4a5a",
    zIndex: 100,
    ...menuStyle,
  };

  const optionBase: BevyStyle = {
    padding: 8,
    backgroundColor: "#2a2a3a",
    ...optionStyle,
  };

  return (
    <Node style={{ position: "relative", flexDirection: "column" }}>
      <Button
        {...handlers}
        style={triggerStyle}
        onClick={() => {
          if (!disabled) {
            setOpen((v) => !v);
          }
        }}
      >
        <Text style={{ fontSize: 16, color: "#ffffff" }}>{display}</Text>
        <Text style={{ fontSize: 12, color: "#aaaaaa" }}>
          {open ? "▲" : "▼"}
        </Text>
      </Button>
      {open ? (
        <Node style={menu}>
          {options.map((opt) => {
            const isSelected = opt.value === value;
            return (
              <Button
                key={opt.value}
                style={{
                  ...optionBase,
                  backgroundColor: isSelected ? "#3a3a5a" : "#2a2a3a",
                }}
                onClick={() => {
                  onChange(opt.value);
                  setOpen(false);
                }}
              >
                <Text style={{ fontSize: 16, color: "#ffffff" }}>
                  {opt.label}
                </Text>
              </Button>
            );
          })}
        </Node>
      ) : null}
    </Node>
  );
}
