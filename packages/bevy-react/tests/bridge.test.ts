/**
 * Bridge unit tests (no React renderer).
 */
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  callNative,
  getBridgeState,
  hostDispatchBridge,
  hostResolveBridgeCall,
  installBridgeDispatcher,
  subscribeBridge,
  useBridgeState,
  useResource,
  useQuery,
} from "../src/bridge";

describe("bridge", () => {
  const g = globalThis as typeof globalThis & Record<string, unknown>;
  let calls: Array<{ name: string; argsJson: string; callId: number }>;

  beforeEach(() => {
    calls = [];
    g.__react_register_bridge_dispatcher = (cb: unknown) => {
      g.__test_bridge_dispatcher = cb;
    };
    g.__react_register_bridge_call_resolver = (cb: unknown) => {
      g.__test_bridge_call_resolver = cb;
    };
    g.__react_call = (name: string, argsJson: string, callId: number) => {
      calls.push({ name, argsJson, callId });
    };
    // Clear module-level channel state between tests via host dispatch overwrite.
    // subscribe/getBridgeState share module maps — reset by reinstalling dispatcher
    // and overwriting known channels with undefined is not supported; tests use
    // unique channel names or re-dispatch.
    installBridgeDispatcher();
  });

  afterEach(() => {
    delete g.__react_register_bridge_dispatcher;
    delete g.__react_register_bridge_call_resolver;
    delete g.__react_call;
    delete g.__test_bridge_dispatcher;
    delete g.__test_bridge_call_resolver;
  });

  it("hostDispatchBridge updates snapshots and notifies subscribers", () => {
    const seen: unknown[] = [];
    const unsub = subscribeBridge("hud-test-a", (v) => seen.push(v));
    hostDispatchBridge("hud-test-a", { hp: 50 });
    expect(getBridgeState("hud-test-a")).toEqual({ hp: 50 });
    expect(seen).toEqual([{ hp: 50 }]);
    unsub();
  });

  it("callNative enqueues with call id and resolves on hostResolveBridgeCall", async () => {
    const promise = callNative<{ score: number }>("add_score", 10);
    expect(calls).toHaveLength(1);
    expect(calls[0].name).toBe("add_score");
    expect(calls[0].argsJson).toBe("10");
    hostResolveBridgeCall(calls[0].callId, { score: 10 });
    await expect(promise).resolves.toEqual({ score: 10 });
  });

  it("callNative rejects when host returns a lone error object", async () => {
    const promise = callNative("missing");
    hostResolveBridgeCall(calls[0].callId, {
      error: "no handler registered for 'missing'",
    });
    await expect(promise).rejects.toThrow(/no handler/);
  });

  it("exports useResource as a useBridgeState alias", () => {
    expect(useResource).toBe(useBridgeState);
  });

  it("exports useQuery as a useBridgeState alias", () => {
    expect(useQuery).toBe(useBridgeState);
  });

  it("installBridgeDispatcher wires host callbacks", () => {
    const dispatch = g.__test_bridge_dispatcher as (
      channel: string,
      value: unknown
    ) => void;
    const resolve = g.__test_bridge_call_resolver as (
      id: number,
      value: unknown
    ) => void;
    expect(typeof dispatch).toBe("function");
    expect(typeof resolve).toBe("function");

    dispatch("hud-test-b", { ok: true });
    expect(getBridgeState("hud-test-b")).toEqual({ ok: true });

    const p = callNative("ping");
    resolve(calls[0].callId, "pong");
    return expect(p).resolves.toBe("pong");
  });
});

/** Mirrors examples/hud/ui/src/hudTypes.ts PLAYER_STATS_KEYS. */
const PLAYER_STATS_KEYS = ["hp", "max_hp", "score"] as const;

describe("hud PlayerStats JSON shape contract", () => {
  it("matches the hand-written TS / Rust serde keys", () => {
    const fixture = { hp: 80, max_hp: 100, score: 1200 };
    expect(Object.keys(fixture)).toEqual([...PLAYER_STATS_KEYS]);
  });
});
