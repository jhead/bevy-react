/**
 * Host → JS event bridge.
 *
 * Rust registers `__react_register_event_dispatcher` / `__react_flush_events`.
 * Call `installEventDispatcher()` once (from `createBevyApp`) so the host can
 * deliver structured events without eval/import string interpolation.
 *
 * Reconciler note: import and keep `dispatchEvent` able to handle the event
 * types listed below (press/release/keyup + richer payloads). See /tmp/epic3-needs.txt.
 */

import { dispatchEvent } from "./reconciler";

export type PointerEventPayload = {
  x?: number;
  y?: number;
  normalized?: boolean;
  cursorOver?: boolean;
};

export type KeyboardEventPayload = {
  key: string;
  shiftKey?: boolean;
  ctrlKey?: boolean;
  altKey?: boolean;
  metaKey?: boolean;
  repeat?: boolean;
  text?: string;
};

export type HostEventPayload = PointerEventPayload | KeyboardEventPayload | null;

type EventDispatcher = (
  rootId: string,
  nodeId: number,
  eventType: string,
  payload: HostEventPayload
) => void;

declare function __react_register_event_dispatcher(callback: EventDispatcher): void;

/**
 * Forward host events into the reconciler's instance map.
 * Event type names match what Rust enqueues: click, press, release, focus, blur,
 * mouseenter, mouseleave, keydown, keyup.
 */
export function hostDispatchEvent(
  _rootId: string,
  nodeId: number,
  eventType: string,
  payload: HostEventPayload
): void {
  // Map host names to reconciler prop handlers until reconciler gains native cases.
  const reconcilerEvent =
    eventType === "press"
      ? "press"
      : eventType === "release"
        ? "release"
        : eventType;

  dispatchEvent(
    nodeId,
    reconcilerEvent,
    (payload ?? undefined) as { key: string } | undefined
  );
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
