import { useState, useCallback, type ReactNode } from "react";
import type { BevyStyle, KeyboardEventData } from "../types";
import { Node, Text } from "../components";

/**
 * Props for the TextInput component
 */
export interface TextInputProps {
    /** Current value of the input */
    value: string;
    /** Callback when value changes */
    onChange: (value: string) => void;
    /** Placeholder text when empty */
    placeholder?: string;
    /** Style for the container */
    style?: BevyStyle;
    /** Style for the text */
    textStyle?: BevyStyle & { fontSize?: number; color?: string };
    /** Style for placeholder text */
    placeholderStyle?: BevyStyle & { fontSize?: number; color?: string };
  }
  
  /**
   * Convert Bevy KeyCode string to character
   */
  function keyCodeToChar(keyCode: string, shiftHeld: boolean): string | null {
    // Handle letter keys (e.g., "KeyA" -> "a" or "A")
    if (keyCode.startsWith("Key")) {
      const letter = keyCode.slice(3).toLowerCase();
      return shiftHeld ? letter.toUpperCase() : letter;
    }
  
    // Handle digit keys (e.g., "Digit1" -> "1")
    if (keyCode.startsWith("Digit")) {
      return keyCode.slice(5);
    }
  
    // Handle numpad keys
    if (keyCode.startsWith("Numpad")) {
      const num = keyCode.slice(6);
      if (num.length === 1 && num >= "0" && num <= "9") {
        return num;
      }
    }
  
    // Handle special characters
    switch (keyCode) {
      case "Space":
        return " ";
      case "Minus":
        return shiftHeld ? "_" : "-";
      case "Equal":
        return shiftHeld ? "+" : "=";
      case "BracketLeft":
        return shiftHeld ? "{" : "[";
      case "BracketRight":
        return shiftHeld ? "}" : "]";
      case "Backslash":
        return shiftHeld ? "|" : "\\";
      case "Semicolon":
        return shiftHeld ? ":" : ";";
      case "Quote":
        return shiftHeld ? '"' : "'";
      case "Comma":
        return shiftHeld ? "<" : ",";
      case "Period":
        return shiftHeld ? ">" : ".";
      case "Slash":
        return shiftHeld ? "?" : "/";
      case "Backquote":
        return shiftHeld ? "~" : "`";
      default:
        return null;
    }
  }
  
  /**
   * Text input component with keyboard handling.
   * Renders a focusable container with text display and cursor.
   */
  export function TextInput({
    value,
    onChange,
    placeholder = "",
    style,
    textStyle,
    placeholderStyle,
  }: TextInputProps): ReactNode {
    const [isFocused, setIsFocused] = useState(false);
    const [showCursor, _] = useState(true);
  
    const handleKeyDown = useCallback(
      (event: KeyboardEventData) => {
        const key = event.key;
  
        if (key === "Backspace") {
          onChange(value.slice(0, -1));
          return;
        }
  
        if (key === "Escape") {
          setIsFocused(false);
          return;
        }
  
        // Convert key code to character
        // TODO: Track shift state properly - for now assume no shift
        const char = keyCodeToChar(key, false);
        if (char) {
          onChange(value + char);
        }
      },
      [value, onChange]
    );
  
    const displayValue = value || "";
    const showPlaceholder = !value && placeholder;
  
    const containerStyle: BevyStyle = {
      flexDirection: "row",
      alignItems: "center",
      padding: 8,
      backgroundColor: isFocused ? "#3a3a4a" : "#2a2a3a",
      borderWidth: 2,
      borderColor: isFocused ? "#6a6aff" : "#4a4a5a",
      minWidth: 120,
      minHeight: 36,
      ...style,
    };
  
    const defaultTextStyle: BevyStyle & { fontSize?: number; color?: string } = {
      fontSize: 16,
      color: "#ffffff",
      ...textStyle,
    };
  
    const defaultPlaceholderStyle: BevyStyle & { fontSize?: number; color?: string } = {
      fontSize: 16,
      color: "#666666",
      ...placeholderStyle,
    };
  
    console.log("TextInput", { showPlaceholder, displayValue, isFocused, showCursor });
  
    return (
      <bevy-text-input
        style={containerStyle}
        onFocus={() => setIsFocused(true)}
        onBlur={() => setIsFocused(false)}
        onKeyDown={handleKeyDown}
      >
        {/* Cursor before text - shown when focused with placeholder */}
        <Node
          style={{
            width: isFocused && showCursor && showPlaceholder ? 2 : 0,
            height: defaultTextStyle.fontSize ?? 16,
            backgroundColor: "#ffffff",
            marginRight: isFocused && showCursor && showPlaceholder ? 4 : 0,
          }}
        />
        {showPlaceholder ? (
          <Text style={defaultPlaceholderStyle}>{placeholder}</Text>
        ) : (
          <Text style={defaultTextStyle}>{displayValue}</Text>
        )}
        {/* Cursor after text - shown when focused with actual content */}
        <Node
          style={{
            width: isFocused && showCursor && !showPlaceholder ? 2 : 0,
            height: defaultTextStyle.fontSize ?? 16,
            backgroundColor: "#ffffff",
            marginLeft: isFocused && showCursor && !showPlaceholder ? 1 : 0,
          }}
        />
      </bevy-text-input>
    );
  }
    