# Rust ↔ React data bridge

Public API for pushing ECS/game state into React and calling registered Rust functions from JS. Supports named JSON channels, ECS-backed resource stores, and Promise-returning typed commands.

## Overview

```
Bevy system                              React (Boa)
─────────────                            ───────────
ReactBridge::publish("hud", …) ──flush──► useBridgeState("hud")
ReactBridge::register_resource_store::<T>("hud")
                               ──flush──► useResource("hud")
ReactBridge::register("fn", …) ◄──call── callNative("fn", args) → Promise
```

State delivery reuses the same host→JS flush pattern as the event queue: Bevy marks channels dirty, then `flush_react_bridge` runs `__react_flush_bridge()` on the JS thread (after syncing registered resource stores). JS→Rust calls enqueue on the JS thread, run on the Bevy main thread with `&mut World`, and return values resolve the matching JS promise on the next flush.

## Rust: push state (manual)

```rust
use bevy::prelude::*;
use bevy_react::ReactBridge;
use serde::Serialize;

#[derive(Serialize, Clone)]
struct HudState {
    hp: i32,
    score: u32,
}

fn sync_hud(bridge: Res<ReactBridge>, /* …query game state… */) {
    bridge.publish(
        "hud",
        HudState {
            hp: 80,
            score: 1200,
        },
    );
}
```

`ReactPlugin` already inserts `ReactBridge` and schedules `flush_react_bridge` each frame.

## Rust: ECS resource stores

Prefer registering a serializable [`Resource`] once instead of manually publishing every frame:

```rust
#[derive(Resource, Clone, Serialize)]
struct PlayerStats {
    hp: i32,
    max_hp: i32,
    score: u32,
}

fn setup_bridge(bridge: Res<ReactBridge>) {
    bridge.register_resource_store::<PlayerStats>("hud");
}
```

Each frame, `flush_react_bridge` snapshots stores when the resource is added/changed (or when the channel has never been published) and feeds the existing dirty/flush path. You can still call `publish` for derived/ad-hoc channels alongside stores.

## Rust: register JS-callable functions

Handlers run inside `process_react_bridge_calls` (exclusive `World` access). **Return values are delivered to JS** and resolve the `callNative` Promise:

```rust
fn setup_bridge(bridge: Res<ReactBridge>) {
    bridge.register_resource_store::<PlayerStats>("hud");

    bridge.register("add_score", |world, args| {
        let points = args.as_i64().unwrap_or(0) as u32;
        let score = if let Some(mut stats) = world.get_resource_mut::<PlayerStats>() {
            stats.score = stats.score.saturating_add(points);
            stats.score
        } else {
            0
        };
        // Resource-store sync will also flush the updated PlayerStats.
        serde_json::json!({ "score": score })
    });
}
```

## TypeScript: subscribe and call

```tsx
import {
  createBevyApp,
  useResource,
  useBridgeState,
  callNative,
  View,
  Text,
} from "bevy-react";

type Hud = { hp: number; max_hp: number; score: number };

function HudOverlay() {
  // Resource store (same channel as register_resource_store)
  const hud = useResource<Hud>("hud", { hp: 0, max_hp: 100, score: 0 });

  // Selector pattern — only re-render when score identity changes
  const score = useBridgeState("hud", { hp: 0, max_hp: 100, score: 0 }, (s) => s.score);

  return (
    <View style={{ position: "absolute", top: 16, left: 16 }}>
      <Text>{`HP ${hud.hp}`}</Text>
      <Text>{`Score ${score}`}</Text>
      <View
        onClick={() => {
          void callNative<{ score: number }>("add_score", 10).then((r) => {
            console.log("new score", r.score);
          });
        }}
        style={{ padding: 8, backgroundColor: "#333" }}
      >
        <Text>+10</Text>
      </View>
    </View>
  );
}

export default createBevyApp(<HudOverlay />);
```

Lower-level helpers:

| API | Role |
|---|---|
| `installBridgeDispatcher()` | Wire host flush → channel store + call resolver (from `createBevyApp`) |
| `subscribeBridge(channel, listener)` | Imperative subscription |
| `getBridgeState(channel)` | Latest snapshot |
| `callNative(name, args?)` | Enqueue `__react_call`; returns `Promise<T>` |
| `useBridgeState(channel, initial)` | React hook via `useSyncExternalStore` |
| `useBridgeState(channel, initial, selector)` | Derived slice with `Object.is` caching |
| `useResource(storeKey, initial)` | Alias of `useBridgeState` for resource stores |

## Native globals

| Global | Direction |
|---|---|
| `__react_register_bridge_dispatcher(cb)` | JS registers `(channel, value) => void` |
| `__react_register_bridge_call_resolver(cb)` | JS registers `(callId, value) => void` |
| `__react_flush_bridge()` | Host drains dirty channels + call results |
| `__react_call(name, argsJson, callId)` | JS enqueues a Rust handler call |

## Codegen path (scaffold)

Full `specta` / `ts-rs` codegen is **not** wired yet. For now:

1. Define a `Serialize` resource in Rust (e.g. `PlayerStats` in `examples/hud`).
2. Mirror the JSON shape in TypeScript (`examples/hud/ui/src/hudTypes.ts`).
3. Keep shapes honest with a Rust unit test (`player_stats_json_shape_matches_ts_contract` in `bridge.rs`) and the `PLAYER_STATS_KEYS` constant on the TS side.

**Next steps for full codegen:** add an optional `ts-rs` / `specta` feature that emits `.ts` from annotated Rust types into `packages/bevy-react` or the example UI, and generate typed `callNative` wrappers from `register` metadata. A future `useQuery`-style API can sit on top of stores + selectors once codegen lands.

## Status

| Area | Status |
|---|---|
| Manual `publish` + dirty flush | Done |
| `register_resource_store` + per-frame sync | Done |
| `callNative` return values (Promise) | Done |
| `useResource` / selector `useBridgeState` | Done |
| Hand-written parallel types + shape test | Done (hud) |
| specta / ts-rs codegen | TODO |
| `useQuery` / entity queries | TODO |

## Notes

- Do not break or bypass the existing `ReactEventQueue` path; the bridge is a separate channel for app data, not UI events.
- Publish only when values change if you care about flush volume; every `publish` dirties the channel for the next frame. Resource stores already gate on change detection.
- Handlers must be `Send + Sync`. Capture shared state carefully; prefer reading/writing Bevy resources via `&mut World`.
- `callNative` resolves after the Bevy frame that runs `process_react_bridge_calls` and the subsequent `__react_flush_bridge` — typically one frame of latency.
