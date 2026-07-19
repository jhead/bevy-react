/**
 * Rust ↔ React data bridge.
 *
 * Host pushes named JSON channels via `ReactBridge::publish`; JS subscribes with
 * `subscribeBridge` / `useBridgeState`. JS invokes registered Rust handlers with
 * `callNative(name, args)`.
 *
 * Call `installBridgeDispatcher()` once (from `createBevyApp`) so state flushes
 * reach subscribers.
 */

import { useSyncExternalStore } from "react";

type BridgeDispatcher = (channel: string, value: unknown) => void;
type BridgeListener = (value: unknown) => void;

const listeners = new Map<string, Set<BridgeListener>>();
const snapshots = new Map<string, unknown>();

declare function __react_register_bridge_dispatcher(callback: BridgeDispatcher): void;
declare function __react_call(name: string, argsJson: string): void;

function notify(channel: string, value: unknown): void {
  snapshots.set(channel, value);
  const set = listeners.get(channel);
  if (!set) {
    return;
  }
  for (const listener of set) {
    listener(value);
  }
}

/**
 * Forward host state updates into the in-memory channel store.
 */
export function hostDispatchBridge(channel: string, value: unknown): void {
  notify(channel, value);
}

/**
 * Register the host→JS bridge dispatcher. Safe to call multiple times.
 */
export function installBridgeDispatcher(): void {
  if (typeof __react_register_bridge_dispatcher !== "function") {
    console.warn(
      "[bevy-react] __react_register_bridge_dispatcher is not available yet"
    );
    return;
  }
  __react_register_bridge_dispatcher(hostDispatchBridge);
}

/**
 * Subscribe to a named bridge channel. Returns an unsubscribe function.
 * If a snapshot already exists, the listener is invoked immediately.
 */
export function subscribeBridge(
  channel: string,
  listener: BridgeListener
): () => void {
  let set = listeners.get(channel);
  if (!set) {
    set = new Set();
    listeners.set(channel, set);
  }
  set.add(listener);

  if (snapshots.has(channel)) {
    listener(snapshots.get(channel));
  }

  return () => {
    set!.delete(listener);
    if (set!.size === 0) {
      listeners.delete(channel);
    }
  };
}

/**
 * Latest snapshot for a channel, if any has been published.
 */
export function getBridgeState<T = unknown>(channel: string): T | undefined {
  return snapshots.get(channel) as T | undefined;
}

/**
 * Invoke a Rust handler registered with `ReactBridge::register`.
 * Fire-and-forget: the host runs the handler on the next Bevy frame.
 */
export function callNative(name: string, args?: unknown): void {
  if (typeof __react_call !== "function") {
    console.warn("[bevy-react] __react_call is not available yet");
    return;
  }
  __react_call(name, JSON.stringify(args ?? null));
}

/**
 * React hook that mirrors a bridge channel into component state.
 */
export function useBridgeState<T>(channel: string, initial: T): T {
  return useSyncExternalStore(
    (onStoreChange) =>
      subscribeBridge(channel, () => {
        onStoreChange();
      }),
    () => (getBridgeState<T>(channel) ?? initial) as T,
    () => initial
  );
}
