import { useState, useCallback, useEffect, type ReactNode } from "react";
import type { BevyStyle, KeyboardEventData } from "../types";
import { Node, Text } from "./Intrinsics";

/** In-process clipboard for Ctrl/Cmd+C/X/V (Boa has no navigator.clipboard). */
let processClipboard = "";

const CURSOR_BLINK_MS = 530;

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
  /** Style for the selection highlight background */
  selectionStyle?: BevyStyle;
}

function clamp(n: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, n));
}

/**
 * Text input with caret, blink, Home/End/arrows, basic selection, and
 * in-process clipboard (Ctrl/Cmd+C/X/V/A). Uses host logical `event.key`.
 */
export function TextInput({
  value,
  onChange,
  placeholder = "",
  style,
  textStyle,
  placeholderStyle,
  selectionStyle,
}: TextInputProps): ReactNode {
  const [isFocused, setIsFocused] = useState(false);
  const [cursor, setCursor] = useState(() => value.length);
  /** Selection anchor; `null` means no active selection range. */
  const [anchor, setAnchor] = useState<number | null>(null);
  const [showCursor, setShowCursor] = useState(true);
  /** Bumped to restart the blink interval after caret moves / edits. */
  const [blinkEpoch, setBlinkEpoch] = useState(0);

  // Keep caret in range when the controlled value shrinks.
  useEffect(() => {
    setCursor((c) => clamp(c, 0, value.length));
    setAnchor((a) => (a == null ? null : clamp(a, 0, value.length)));
  }, [value.length]);

  // Cursor blink while focused; `blinkEpoch` restarts the timer after edits.
  useEffect(() => {
    if (!isFocused) {
      setShowCursor(false);
      return;
    }
    setShowCursor(true);
    const id = setInterval(() => {
      setShowCursor((v) => !v);
    }, CURSOR_BLINK_MS);
    return () => clearInterval(id);
  }, [isFocused, blinkEpoch]);

  const bumpBlink = useCallback(() => {
    setBlinkEpoch((n) => n + 1);
    setShowCursor(true);
  }, []);

  const selStart =
    anchor == null ? cursor : Math.min(anchor, cursor);
  const selEnd = anchor == null ? cursor : Math.max(anchor, cursor);
  const hasSelection = anchor != null && selStart !== selEnd;

  const replaceRange = useCallback(
    (start: number, end: number, insert: string) => {
      const next = value.slice(0, start) + insert + value.slice(end);
      const nextCursor = start + insert.length;
      onChange(next);
      setCursor(nextCursor);
      setAnchor(null);
      bumpBlink();
    },
    [value, onChange, bumpBlink]
  );

  const moveCaret = useCallback(
    (next: number, extend: boolean) => {
      const clamped = clamp(next, 0, value.length);
      if (extend) {
        setAnchor((a) => (a == null ? cursor : a));
        setCursor(clamped);
      } else {
        setAnchor(null);
        setCursor(clamped);
      }
      bumpBlink();
    },
    [value.length, cursor, bumpBlink]
  );

  const handleKeyDown = useCallback(
    (event: KeyboardEventData) => {
      const key = event.key;
      const mod = !!(event.ctrlKey || event.metaKey);
      const shift = !!event.shiftKey;

      // Clipboard / select-all (Ctrl or Cmd)
      if (mod && !event.altKey) {
        const lower = key.length === 1 ? key.toLowerCase() : key;
        if (lower === "a") {
          setAnchor(0);
          setCursor(value.length);
          bumpBlink();
          return;
        }
        if (lower === "c") {
          if (hasSelection) {
            processClipboard = value.slice(selStart, selEnd);
          }
          return;
        }
        if (lower === "x") {
          if (hasSelection) {
            processClipboard = value.slice(selStart, selEnd);
            replaceRange(selStart, selEnd, "");
          }
          return;
        }
        if (lower === "v") {
          if (processClipboard) {
            if (hasSelection) {
              replaceRange(selStart, selEnd, processClipboard);
            } else {
              replaceRange(cursor, cursor, processClipboard);
            }
          }
          return;
        }
        // Ignore other modified keys (e.g. Ctrl+Arrow reserved for later)
        return;
      }

      if (event.altKey) {
        return;
      }

      if (key === "Backspace") {
        if (hasSelection) {
          replaceRange(selStart, selEnd, "");
        } else if (cursor > 0) {
          replaceRange(cursor - 1, cursor, "");
        }
        return;
      }

      if (key === "Delete") {
        if (hasSelection) {
          replaceRange(selStart, selEnd, "");
        } else if (cursor < value.length) {
          replaceRange(cursor, cursor + 1, "");
        }
        return;
      }

      if (key === "ArrowLeft") {
        if (!shift && hasSelection) {
          moveCaret(selStart, false);
        } else {
          moveCaret(cursor - 1, shift);
        }
        return;
      }

      if (key === "ArrowRight") {
        if (!shift && hasSelection) {
          moveCaret(selEnd, false);
        } else {
          moveCaret(cursor + 1, shift);
        }
        return;
      }

      if (key === "Home") {
        moveCaret(0, shift);
        return;
      }

      if (key === "End") {
        moveCaret(value.length, shift);
        return;
      }

      if (key === "Escape") {
        setIsFocused(false);
        setAnchor(null);
        return;
      }

      // Ignore navigation / modifiers that aren't printable
      if (
        key === "ArrowUp" ||
        key === "ArrowDown" ||
        key === "Tab" ||
        key === "Enter" ||
        key === "Shift" ||
        key === "Control" ||
        key === "Alt" ||
        key === "Meta"
      ) {
        return;
      }

      const text =
        event.text ?? (key.length === 1 ? key : null);
      if (text) {
        if (hasSelection) {
          replaceRange(selStart, selEnd, text);
        } else {
          replaceRange(cursor, cursor, text);
        }
      }
    },
    [
      value,
      cursor,
      hasSelection,
      selStart,
      selEnd,
      replaceRange,
      moveCaret,
      bumpBlink,
    ]
  );

  const handleFocus = useCallback(() => {
    setIsFocused(true);
    setCursor(value.length);
    setAnchor(null);
    bumpBlink();
  }, [value.length, bumpBlink]);

  const handleBlur = useCallback(() => {
    setIsFocused(false);
    setAnchor(null);
    setShowCursor(false);
  }, []);

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
    overflow: "hidden",
    ...style,
  };

  const defaultTextStyle: BevyStyle & { fontSize?: number; color?: string } = {
    fontSize: 16,
    color: "#ffffff",
    ...textStyle,
  };

  const defaultPlaceholderStyle: BevyStyle & {
    fontSize?: number;
    color?: string;
  } = {
    fontSize: 16,
    color: "#666666",
    ...placeholderStyle,
  };

  const fontSize = defaultTextStyle.fontSize ?? 16;
  const cursorVisible = isFocused && showCursor && !hasSelection;

  const before = hasSelection
    ? displayValue.slice(0, selStart)
    : displayValue.slice(0, cursor);
  const selected = hasSelection
    ? displayValue.slice(selStart, selEnd)
    : "";
  const after = hasSelection
    ? displayValue.slice(selEnd)
    : displayValue.slice(cursor);

  return (
    <bevy-text-input
      style={containerStyle}
      onFocus={handleFocus}
      onBlur={handleBlur}
      onKeyDown={handleKeyDown}
    >
      {showPlaceholder ? (
        <>
          <Node
            style={{
              width: cursorVisible ? 2 : 0,
              height: fontSize,
              backgroundColor: "#ffffff",
              marginRight: cursorVisible ? 2 : 0,
            }}
          />
          <Text style={defaultPlaceholderStyle}>{placeholder}</Text>
        </>
      ) : (
        <>
          {before ? <Text style={defaultTextStyle}>{before}</Text> : null}
          {hasSelection ? (
            <Node
              style={{
                flexDirection: "row",
                alignItems: "center",
                backgroundColor: "#4a6aff",
                ...selectionStyle,
              }}
            >
              <Text
                style={{
                  ...defaultTextStyle,
                  color: "#ffffff",
                }}
              >
                {selected}
              </Text>
            </Node>
          ) : (
            <Node
              style={{
                width: cursorVisible ? 2 : 0,
                height: fontSize,
                backgroundColor: "#ffffff",
                marginLeft: before ? 1 : 0,
                marginRight: after ? 1 : 0,
              }}
            />
          )}
          {after ? <Text style={defaultTextStyle}>{after}</Text> : null}
        </>
      )}
    </bevy-text-input>
  );
}
