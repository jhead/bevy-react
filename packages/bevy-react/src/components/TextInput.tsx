import { useState, useCallback, useEffect, useRef, type ReactNode } from "react";
import type { BevyStyle, KeyboardEventData } from "../types";
import { Node, Text } from "./Intrinsics";

/** In-process clipboard for Ctrl/Cmd+C/X/V (Boa has no navigator.clipboard). */
let processClipboard = "";

const CURSOR_BLINK_MS = 530;

/** Children of the focusable shell must not steal pointer hits / blur focus. */
const PASS_THROUGH: BevyStyle = { pointerEvents: "none" };

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

/** Prefer non-empty `event.text`; empty string must not block `event.key`. */
export function printableFromKeyEvent(event: {
  key: string;
  text?: string | null;
}): string | null {
  const raw = event.text;
  if (raw != null && raw.length > 0) {
    return raw;
  }
  return event.key.length === 1 ? event.key : null;
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

  // Keep latest controlled value / caret in refs so the host-held keydown
  // callback never inserts against a stale closure after the first character.
  const valueRef = useRef(value);
  const cursorRef = useRef(cursor);
  const anchorRef = useRef(anchor);
  valueRef.current = value;
  cursorRef.current = cursor;
  anchorRef.current = anchor;

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
      const current = valueRef.current;
      const next = current.slice(0, start) + insert + current.slice(end);
      const nextCursor = start + insert.length;
      // Update before parent re-renders — discrete flush may still lag a tick.
      valueRef.current = next;
      onChange(next);
      setCursor(nextCursor);
      cursorRef.current = nextCursor;
      setAnchor(null);
      anchorRef.current = null;
      bumpBlink();
    },
    [onChange, bumpBlink]
  );

  const moveCaret = useCallback(
    (next: number, extend: boolean) => {
      const len = valueRef.current.length;
      const clamped = clamp(next, 0, len);
      if (extend) {
        setAnchor((a) => {
          const nextAnchor = a == null ? cursorRef.current : a;
          anchorRef.current = nextAnchor;
          return nextAnchor;
        });
        setCursor(clamped);
        cursorRef.current = clamped;
      } else {
        setAnchor(null);
        anchorRef.current = null;
        setCursor(clamped);
        cursorRef.current = clamped;
      }
      bumpBlink();
    },
    [bumpBlink]
  );

  /** Keys that already ran editing logic on keydown (skip duplicate keyup). */
  const handledOnKeyDownRef = useRef(new Set<string>());

  const handleKeyEvent = useCallback(
    (event: KeyboardEventData, phase: "keydown" | "keyup") => {
      const key = event.key;
      if (phase === "keyup") {
        if (handledOnKeyDownRef.current.has(key)) {
          handledOnKeyDownRef.current.delete(key);
          return;
        }
        // Host sometimes delivers only keyup after the first glyph — still edit.
      } else {
        handledOnKeyDownRef.current.add(key);
      }

      const mod = !!(event.ctrlKey || event.metaKey);
      const shift = !!event.shiftKey;
      const valueNow = valueRef.current;
      const cursorNow = cursorRef.current;
      const anchorNow = anchorRef.current;
      const selA =
        anchorNow == null ? cursorNow : Math.min(anchorNow, cursorNow);
      const selB =
        anchorNow == null ? cursorNow : Math.max(anchorNow, cursorNow);
      const selecting = anchorNow != null && selA !== selB;

      // Clipboard / select-all (Ctrl or Cmd)
      if (mod && !event.altKey) {
        const lower = key.length === 1 ? key.toLowerCase() : key;
        if (lower === "a") {
          setAnchor(0);
          anchorRef.current = 0;
          setCursor(valueNow.length);
          cursorRef.current = valueNow.length;
          bumpBlink();
          return;
        }
        if (lower === "c") {
          if (selecting) {
            processClipboard = valueNow.slice(selA, selB);
          }
          return;
        }
        if (lower === "x") {
          if (selecting) {
            processClipboard = valueNow.slice(selA, selB);
            replaceRange(selA, selB, "");
          }
          return;
        }
        if (lower === "v") {
          if (processClipboard) {
            if (selecting) {
              replaceRange(selA, selB, processClipboard);
            } else {
              replaceRange(cursorNow, cursorNow, processClipboard);
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
        if (selecting) {
          replaceRange(selA, selB, "");
        } else if (cursorNow > 0) {
          replaceRange(cursorNow - 1, cursorNow, "");
        }
        return;
      }

      if (key === "Delete") {
        if (selecting) {
          replaceRange(selA, selB, "");
        } else if (cursorNow < valueNow.length) {
          replaceRange(cursorNow, cursorNow + 1, "");
        }
        return;
      }

      if (key === "ArrowLeft") {
        if (!shift && selecting) {
          moveCaret(selA, false);
        } else {
          moveCaret(cursorNow - 1, shift);
        }
        return;
      }

      if (key === "ArrowRight") {
        if (!shift && selecting) {
          moveCaret(selB, false);
        } else {
          moveCaret(cursorNow + 1, shift);
        }
        return;
      }

      if (key === "Home") {
        moveCaret(0, shift);
        return;
      }

      if (key === "End") {
        moveCaret(valueNow.length, shift);
        return;
      }

      if (key === "Escape") {
        setIsFocused(false);
        setAnchor(null);
        anchorRef.current = null;
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

      const text = printableFromKeyEvent(event);
      if (text) {
        if (selecting) {
          replaceRange(selA, selB, text);
        } else {
          replaceRange(cursorNow, cursorNow, text);
        }
      }
    },
    [replaceRange, moveCaret, bumpBlink]
  );

  const handleKeyDown = useCallback(
    (event: KeyboardEventData) => handleKeyEvent(event, "keydown"),
    [handleKeyEvent]
  );

  const handleKeyUp = useCallback(
    (event: KeyboardEventData) => handleKeyEvent(event, "keyup"),
    [handleKeyEvent]
  );

  const handleFocus = useCallback(() => {
    setIsFocused(true);
    setCursor(valueRef.current.length);
    cursorRef.current = valueRef.current.length;
    setAnchor(null);
    anchorRef.current = null;
    bumpBlink();
  }, [bumpBlink]);

  const handleBlur = useCallback(() => {
    setIsFocused(false);
    setAnchor(null);
    anchorRef.current = null;
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
    pointerEvents: "none",
  };

  const defaultPlaceholderStyle: BevyStyle & {
    fontSize?: number;
    color?: string;
  } = {
    fontSize: 16,
    color: "#666666",
    ...placeholderStyle,
    pointerEvents: "none",
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

  const cursorStyle: BevyStyle = {
    ...PASS_THROUGH,
    width: cursorVisible ? 2 : 0,
    height: fontSize,
    backgroundColor: "#ffffff",
  };

  return (
    <bevy-text-input
      style={containerStyle}
      onFocus={handleFocus}
      onBlur={handleBlur}
      onKeyDown={handleKeyDown}
      onKeyUp={handleKeyUp}
    >
      {showPlaceholder ? (
        <>
          <Node
            style={{
              ...cursorStyle,
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
                ...PASS_THROUGH,
                flexDirection: "row",
                alignItems: "center",
                backgroundColor: "#4a6aff",
                ...selectionStyle,
                pointerEvents: "none",
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
                ...cursorStyle,
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
