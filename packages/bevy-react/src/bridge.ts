/**
 * Rust ↔ React data bridge.
 *
 * Host pushes named JSON channels via `ReactBridge::publish` / resource stores;
 * JS subscribes with `subscribeBridge` / `useBridgeState` / `useResource`.
 * JS invokes registered Rust handlers with `callNative(name, args)` (Promise).
 *
 * Call `installBridgeDispatcher()` once (from `createBevyApp`) so state flushes
 * and call results reach subscribers / resolvers.
 */

import { useRef, useSyncExternalStore } from "react";

type BridgeDispatcher = (channel: string, value: unknown) => void;
type BridgeCallResolver = (callId: number, value: unknown) => void;
type BridgeListener = (value: unknown) => void;

const listeners = new Map<string, Set<BridgeListener>>();
const snapshots = new Map<string, unknown>();

let nextCallId = 1;
const pendingCalls = new Map<
  number,
  { resolve: (value: unknown) => void; reject: (reason?: unknown) => void }
>();

declare function __react_register_bridge_dispatcher(
  callback: BridgeDispatcher
): void;
declare function __react_register_bridge_call_resolver(
  callback: BridgeCallResolver
): void;
declare function __react_call(
  name: string,
  argsJson: string,
  callId: number
): void;

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
 * Resolve a pending `callNative` promise when the host flushes a call result.
 */
export function hostResolveBridgeCall(callId: number, value: unknown): void {
  const pending = pendingCalls.get(callId);
  if (!pending) {
    return;
  }
  pendingCalls.delete(callId);
  if (
    value !== null &&
    typeof value === "object" &&
    "error" in value &&
    typeof (value as { error: unknown }).error === "string" &&
    Object.keys(value as object).length === 1
  ) {
    pending.reject(new Error((value as { error: string }).error));
    return;
  }
  pending.resolve(value);
}

/**
 * Register the host→JS bridge dispatcher and call resolver. Safe to call multiple times.
 */
export function installBridgeDispatcher(): void {
  if (typeof __react_register_bridge_dispatcher !== "function") {
    console.warn(
      "[bevy-react] __react_register_bridge_dispatcher is not available yet"
    );
  } else {
    __react_register_bridge_dispatcher(hostDispatchBridge);
  }

  if (typeof __react_register_bridge_call_resolver !== "function") {
    console.warn(
      "[bevy-react] __react_register_bridge_call_resolver is not available yet"
    );
  } else {
    __react_register_bridge_call_resolver(hostResolveBridgeCall);
  }
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
 * Returns a Promise that resolves with the handler's JSON return value
 * after the next Bevy frame flush.
 */
export function callNative<T = unknown>(
  name: string,
  args?: unknown
): Promise<T> {
  if (typeof __react_call !== "function") {
    console.warn("[bevy-react] __react_call is not available yet");
    return Promise.reject(new Error("__react_call is not available"));
  }

  const callId = nextCallId++;
  return new Promise<T>((resolve, reject) => {
    pendingCalls.set(callId, {
      resolve: (value) => resolve(value as T),
      reject,
    });
    __react_call(name, JSON.stringify(args ?? null), callId);
  });
}

/**
 * React hook that mirrors a bridge channel into component state.
 *
 * Pass an optional `selector` to subscribe to a derived slice. The selected
 * value is cached with `Object.is` against the previous selection so
 * referentially equal results do not retrigger renders.
 */
export function useBridgeState<T>(channel: string, initial: T): T;
export function useBridgeState<T, S>(
  channel: string,
  initial: T,
  selector: (state: T) => S
): S;
export function useBridgeState<T, S = T>(
  channel: string,
  initial: T,
  selector?: (state: T) => S
): S {
  const select = selector ?? ((state: T) => state as unknown as S);
  const cacheRef = useRef<{ raw: T; selected: S } | null>(null);

  return useSyncExternalStore(
    (onStoreChange) =>
      subscribeBridge(channel, () => {
        onStoreChange();
      }),
    () => {
      const raw = (getBridgeState<T>(channel) ?? initial) as T;
      const cached = cacheRef.current;
      if (cached && Object.is(cached.raw, raw)) {
        return cached.selected;
      }
      const selected = select(raw);
      if (cached && Object.is(cached.selected, selected)) {
        cacheRef.current = { raw, selected: cached.selected };
        return cached.selected;
      }
      cacheRef.current = { raw, selected };
      return selected;
    },
    () => select(initial)
  );
}

/**
 * Subscribe to an ECS-backed resource store published under `storeKey`.
 *
 * Alias of {@link useBridgeState} for the `register_resource_store` path.
 */
export const useResource: typeof useBridgeState = useBridgeState;
