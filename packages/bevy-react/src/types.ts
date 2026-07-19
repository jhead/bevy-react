import type { ReactNode, Ref } from "react";
import type { BevyHostInstance } from "./entity";

/**
 * Style properties that map to Bevy's UI Style component.
 * Values can be:
 * - Numbers (treated as pixels)
 * - Strings like "100px", "50%", "auto"
 */
export interface BevyStyle {
  // Size
  width?: string | number;
  height?: string | number;
  minWidth?: string | number;
  minHeight?: string | number;
  maxWidth?: string | number;
  maxHeight?: string | number;
  /** Width / height ratio (`1.5` or `"16/9"`). */
  aspectRatio?: string | number;

  // Position
  position?: "relative" | "absolute";
  left?: string | number;
  right?: string | number;
  top?: string | number;
  bottom?: string | number;

  // Flexbox
  flexDirection?: "row" | "column" | "rowReverse" | "columnReverse";
  flexWrap?: "noWrap" | "wrap" | "wrapReverse";
  flexGrow?: number;
  flexShrink?: number;
  flexBasis?: string | number;
  alignItems?: "start" | "end" | "center" | "baseline" | "stretch";
  alignSelf?: "auto" | "start" | "end" | "center" | "baseline" | "stretch";
  alignContent?:
    | "start"
    | "end"
    | "center"
    | "stretch"
    | "spaceBetween"
    | "spaceAround"
    | "spaceEvenly";
  justifyContent?:
    | "start"
    | "end"
    | "center"
    | "spaceBetween"
    | "spaceAround"
    | "spaceEvenly";
  justifyItems?: "start" | "end" | "center" | "baseline" | "stretch";
  justifySelf?: "auto" | "start" | "end" | "center" | "baseline" | "stretch";

  // CSS Grid
  gridTemplateColumns?: string;
  gridTemplateRows?: string;
  gridAutoColumns?: string;
  gridAutoRows?: string;
  gridAutoFlow?: "row" | "column" | "row dense" | "column dense" | string;
  gridColumn?: string;
  gridRow?: string;
  gridColumnStart?: string | number;
  gridColumnEnd?: string | number;
  gridRowStart?: string | number;
  gridRowEnd?: string | number;

  // Spacing (shorthands accept 1–4 CSS values, e.g. `"8px 16px"`)
  gap?: string | number;
  rowGap?: string | number;
  columnGap?: string | number;
  margin?: string | number;
  marginLeft?: string | number;
  marginRight?: string | number;
  marginTop?: string | number;
  marginBottom?: string | number;
  padding?: string | number;
  paddingLeft?: string | number;
  paddingRight?: string | number;
  paddingTop?: string | number;
  paddingBottom?: string | number;

  // Border
  /** Uniform border width (preferred). Accepted by Rust as alias for `border`. */
  borderWidth?: string | number;
  /** @deprecated Prefer `borderWidth` — kept as alias for Rust `border`. */
  border?: string | number;
  borderTop?: string | number;
  borderRight?: string | number;
  borderBottom?: string | number;
  borderLeft?: string | number;
  borderColor?: string;
  borderTopColor?: string;
  borderRightColor?: string;
  borderBottomColor?: string;
  borderLeftColor?: string;
  /** 1–4 value shorthand (`"8px 16px"`). */
  borderRadius?: string | number;
  borderTopLeftRadius?: string | number;
  borderTopRightRadius?: string | number;
  borderBottomRightRadius?: string | number;
  borderBottomLeftRadius?: string | number;

  // Visual
  backgroundColor?: string;
  /** CSS `linear-gradient(...)` → Bevy `BackgroundGradient` (helper; render wiring may lag). */
  backgroundImage?: string;
  backgroundGradient?: string;
  boxShadow?: string;
  /** CSS text-shadow → Bevy `TextShadow` (e.g. `"2px 3px 0 black"`). */
  textShadow?: string;
  /** 0–1 or percentage string. Bevy has no UiOpacity; multiply into colors in render. */
  opacity?: string | number;
  zIndex?: number;

  // Display / overflow
  display?: "flex" | "none" | "grid" | "block";
  overflow?: "visible" | "clip" | "hidden" | "scroll";
  overflowX?: "visible" | "clip" | "hidden" | "scroll";
  overflowY?: "visible" | "clip" | "hidden" | "scroll";
  /** `content-box` | `padding-box` | `border-box` | length | `"content-box 4px"`. */
  overflowClipMargin?: string;

  // Text (also used on Text style)
  color?: string;
  fontSize?: string | number;
  /** Font asset path (e.g. `"fonts/FiraSans.ttf"`). Generic CSS families ignored. */
  fontFamily?: string;
  textAlign?: "left" | "right" | "center" | "justify" | "start" | "end";
  /** Unitless multiplier or `"24px"`. */
  lineHeight?: string | number;

  /**
   * Pointer hit testing for full-screen / transparent HUD roots.
   * `"none"` → Bevy `Pickable::IGNORE` + `FocusPolicy::Pass` (clicks reach the world).
   * `"auto"` → restore default blocking (omit for default).
   */
  pointerEvents?: "none" | "auto";

  // Image
  objectFit?: "fill" | "contain" | "cover" | "none" | "scale-down" | "stretch" | "auto";
  tint?: string;
  tintColor?: string;

  /**
   * Host-side hover style overrides. Applied in Rust from Bevy `Interaction`
   * or picking `Hovered` (no React round-trip). JS still handles `onClick` /
   * pointer events.
   */
  hover?: BevyStyle;
  /** Host-side pressed / active overrides (`Interaction::Pressed`). */
  pressed?: BevyStyle;
  /** Host-side focused overrides (keyboard / input focus). */
  focused?: BevyStyle;
  /**
   * Host-side checked overrides (Bevy UI `Checked` marker, e.g. checkbox).
   * Merged under hover/pressed: base → checked → focused → hover → pressed.
   */
  checked?: BevyStyle;
  /**
   * Host-side transitions for color/numeric props between interaction states.
   * String: `"backgroundColor 100ms"` or `"backgroundColor 100ms, opacity 200ms"`.
   * Object: `{ backgroundColor: 100 }` (values in milliseconds).
   */
  transition?: string | Record<string, number | string>;
}

/**
 * Props for the <node> element (NodeBundle)
 */
export interface NodeProps {
  style?: BevyStyle;
  children?: ReactNode;
  /**
   * Named Rust bundles to attach to this UI entity (ECS escape hatch).
   * Register appliers with `BundleRegistry::register("Glow", ...)`.
   */
  components?: string[];
  onClick?: (event?: PointerSyntheticEvent | PointerEventData) => void;
  onPress?: (event?: PointerSyntheticEvent | PointerEventData) => void;
  onRelease?: (event?: PointerSyntheticEvent | PointerEventData) => void;
  onHover?: (event?: PointerSyntheticEvent | PointerEventData) => void;
  onMouseEnter?: (event?: PointerSyntheticEvent | PointerEventData) => void;
  onMouseLeave?: (event?: PointerSyntheticEvent | PointerEventData) => void;
  onMouseMove?: (event?: PointerSyntheticEvent | PointerEventData) => void;
  onDrag?: (event?: PointerSyntheticEvent | PointerEventData) => void;
  onWheel?: (event?: WheelSyntheticEvent | WheelEventData) => void;
  onScroll?: (event?: ScrollSyntheticEvent | ScrollEventData) => void;
}

/**
 * Pointer / cursor payload from the host event queue.
 */
export interface PointerEventData {
  x?: number;
  y?: number;
  normalized?: boolean;
  cursorOver?: boolean;
}

/**
 * Keyboard event data (DOM-like logical key + modifiers from the host)
 */
export interface KeyboardEventData {
  key: string;
  shiftKey?: boolean;
  ctrlKey?: boolean;
  altKey?: boolean;
  metaKey?: boolean;
  repeat?: boolean;
  text?: string;
}

/**
 * Mouse-wheel payload from the host (`wheel` events).
 * `deltaMode`: 0 = pixel, 1 = line (DOM `WheelEvent.DOM_DELTA_*`).
 */
export interface WheelEventData {
  deltaX?: number;
  deltaY?: number;
  deltaMode?: number;
}

/**
 * Scroll position payload after the host applies a wheel delta (`scroll` events).
 */
export interface ScrollEventData {
  scrollLeft?: number;
  scrollTop?: number;
  deltaX?: number;
  deltaY?: number;
}

/**
 * Synthetic extras attached by the reconciler for bubbling / `stopPropagation`.
 * Payload fields are spread onto the same object so handlers can read `event.key` / `event.x`.
 */
export interface SyntheticEventExtras {
  type: string;
  target: number;
  currentTarget: number;
  stopPropagation: () => void;
}

export type PointerSyntheticEvent = PointerEventData & SyntheticEventExtras;
export type KeyboardSyntheticEvent = KeyboardEventData & SyntheticEventExtras;
export type WheelSyntheticEvent = WheelEventData & SyntheticEventExtras;
export type ScrollSyntheticEvent = ScrollEventData & SyntheticEventExtras;

/**
 * Props for the <button> element (ButtonBundle with interaction)
 */
export interface ButtonProps extends NodeProps {}

/**
 * Host props for headless `bevy-slider`.
 */
export interface SliderHostProps extends NodeProps {
  value?: number;
  min?: number;
  max?: number;
  step?: number;
  /** Visual axis; Bevy 0.17 drag math remains horizontal. */
  orientation?: "horizontal" | "vertical";
  disabled?: boolean;
  onChange?: (event: ChangeSyntheticEvent | { value: number }) => void;
}

/**
 * Host props for `bevy-slider-thumb`.
 */
export interface SliderThumbHostProps extends NodeProps {}

/**
 * Host props for headless `bevy-checkbox`.
 */
export interface CheckboxHostProps extends NodeProps {
  checked?: boolean;
  disabled?: boolean;
  onChange?: (event: ChangeSyntheticEvent | { value: boolean }) => void;
}

/**
 * Value-change payload from host widgets (`change` events).
 */
export interface ChangeEventData {
  value?: number | boolean | string;
}

export type ChangeSyntheticEvent = ChangeEventData & SyntheticEventExtras;

/**
 * Props for the internal <bevy-text-input> element (focusable container)
 */
export interface TextInputInternalProps extends NodeProps {
  onFocus?: () => void;
  onBlur?: () => void;
  onKeyDown?: (event: KeyboardSyntheticEvent | KeyboardEventData) => void;
  onKeyUp?: (event: KeyboardSyntheticEvent | KeyboardEventData) => void;
}

/**
 * Props for the <text> element (TextBundle)
 */
export interface TextProps {
  children?: ReactNode;
  style?: BevyStyle;
}

/**
 * Props for the <image> element (ImageBundle)
 */
export interface ImageProps {
  src: string;
  style?: BevyStyle;
}

/**
 * Internal instance type used by the reconciler
 */
export interface BevyInstance {
  nodeId: number;
  type: string;
  props: Record<string, unknown>;
  children: BevyInstance[];
  /** Parent node id for event bubbling; unset for container children. */
  parentId?: number;
  /**
   * HostText children folded into this `bevy-text` node's `content`.
   * Avoids a dual-entity bug where CreateNode paints glyphs on the host while
   * UpdateText mutates a sibling HostText entity that never shows.
   */
  textSlots?: BevyTextInstance[];
}

/**
 * Text instance type
 */
export interface BevyTextInstance {
  nodeId: number;
  text: string;
  /** Present when this text is folded into a `bevy-text` host instead of its own entity. */
  textHost?: BevyInstance;
}

/**
 * Internal JSX augmentation for host elements.
 * Users should import the exported components (Node, Button, Text, Image) instead.
 */
type BevyIntrinsicElements = {
  "bevy-node": NodeProps & { ref?: Ref<BevyHostInstance> };
  "bevy-button": ButtonProps & { ref?: Ref<BevyHostInstance> };
  "bevy-slider": SliderHostProps;
  "bevy-slider-thumb": SliderThumbHostProps;
  "bevy-checkbox": CheckboxHostProps;
  "bevy-text": TextProps;
  "bevy-image": ImageProps;
  "bevy-text-input": TextInputInternalProps;
};

declare global {
  namespace JSX {
    interface IntrinsicElements extends BevyIntrinsicElements {}
  }
}

declare module "react" {
  namespace JSX {
    interface IntrinsicElements extends BevyIntrinsicElements {}
  }
}
