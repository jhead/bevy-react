/**
 * Host → JS event bridge.
 *
 * Rust registers `__react_register_event_dispatcher` / `__react_flush_events`.
 * Call `installEventDispatcher()` once (from `createBevyApp`) so the host can
 * deliver structured events without eval/import string interpolation.
 */

import { dispatchEvent } from "./reconciler";
import type {
  KeyboardEventData,
  PointerEventData,
  ScrollEventData,
  WheelEventData,
} from "./types";

export type PointerEventPayload = PointerEventData;
export type KeyboardEventPayload = KeyboardEventData;
export type WheelEventPayload = WheelEventData;
export type ScrollEventPayload = ScrollEventData;

export type HostEventPayload =
  | PointerEventPayload
  | KeyboardEventPayload
  | WheelEventPayload
  | ScrollEventPayload
  | null;

type EventDispatcher = (
  rootId: string,
  nodeId: number,
  eventType: string,
  payload: HostEventPayload
) => void;

declare function __react_register_event_dispatcher(callback: EventDispatcher): void;

/**
 * Forward host events into the reconciler's per-root instance map.
 * Event type names match what Rust enqueues: click, press, release, focus, blur,
 * mouseenter, mouseleave, keydown, keyup, wheel, scroll.
 */
export function hostDispatchEvent(
  rootId: string,
  nodeId: number,
  eventType: string,
  payload: HostEventPayload
): void {
  dispatchEvent(rootId, nodeId, eventType, payload ?? undefined);
}

/**
 * Register the host→JS callback with the Rust native bridge.
 * Safe to call multiple times; last registration wins.
 */
export function installEventDispatcher(): void {
  if (typeof __react_register_event_dispatcher !== "function") {
    console.warn(
      "[bevy-react] __react_register_event_dispatcher is not available yet"
    );
    return;
  }

  __react_register_event_dispatcher(hostDispatchEvent);
}
