# Rust ↔ React data bridge

Public API for pushing ECS/game state into React and calling registered Rust functions from JS. Supports named JSON channels, ECS-backed resource stores, query stores, and Promise-returning typed commands.

## Overview

```
Bevy system                              React (Boa)
─────────────                            ───────────
ReactBridge::publish("hud", …) ──flush──► useBridgeState("hud")
ReactBridge::register_resource_store::<T>("hud")
                               ──flush──► useResource("hud")
ReactBridge::register_query_store("enemies", …)
  + mark_query_dirty / each_frame
                               ──flush──► useQuery("enemies")
ReactBridge::register / register_typed / BridgeCommandSet
                               ◄──call── callNative("fn", args) → Promise
```

State delivery reuses the same host→JS flush pattern as the event queue: Bevy marks channels dirty, then `flush_react_bridge` runs `__react_flush_bridge()` on the JS thread (after syncing registered resource and query stores). JS→Rust calls enqueue on the JS thread, run on the Bevy main thread with `&mut World`, and return values resolve the matching JS promise on the next flush.

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

## Rust: ECS query stores

For entity/component subscriptions, register a named query store with a closure that serializes matching data. The flush path runs the closure when the store is dirty (or has never published):

```rust
#[derive(Component, Clone, Serialize)]
struct Enemy {
    hp: i32,
}

fn setup_bridge(bridge: Res<ReactBridge>) {
    bridge.register_query_store("enemies", |world| {
        let mut q = world.query::<(Entity, &Enemy)>();
        let rows: Vec<_> = q
            .iter(world)
            .map(|(e, enemy)| {
                serde_json::json!({
                    "entity": e.to_bits(),
                    "hp": enemy.hp,
                })
            })
            .collect();
        serde_json::json!(rows)
    });
}

/// Prefer Changed detection: mark dirty only when relevant components move.
fn watch_enemies(
    bridge: Res<ReactBridge>,
    changed: Query<(), Or<(Changed<Enemy>, Added<Enemy>)>>,
    mut removed: RemovedComponents<Enemy>,
) {
    if !changed.is_empty() || removed.read().next().is_some() {
        bridge.mark_query_dirty("enemies");
    }
}
```

For cheap queries, use `register_query_store_each_frame` instead of manual dirty marks. Unchanged JSON is not republished either way.

| API | Role |
|---|---|
| `register_query_store(name, Fn(&mut World) -> Value)` | Snapshot when dirty / first publish |
| `register_query_store_each_frame(…)` | Snapshot every flush (skip if equal) |
| `mark_query_dirty(name)` | Schedule a re-snapshot next flush |
| `unregister_query_store(name)` | Drop the registration |

## Rust: register JS-callable functions

Handlers run inside `process_react_bridge_calls` (exclusive `World` access). **Return values are delivered to JS** and resolve the `callNative` Promise.

Prefer [`BridgeCommandSet`] (or `register_typed`) so command names stay tied to [`BridgeCommandMeta`] used for TypeScript codegen:

```rust
use bevy_react::{BridgeCommandMeta, BridgeCommandSet, ReactBridge};

fn setup_bridge(bridge: Res<ReactBridge>) {
    bridge.register_resource_store::<PlayerStats>("hud");

    BridgeCommandSet::new()
        .command(
            BridgeCommandMeta::new("add_score", "addScore", "number", "{ score: number }"),
            |world, args| {
                let points = args.as_i64().unwrap_or(0) as u32;
                let score = if let Some(mut stats) = world.get_resource_mut::<PlayerStats>() {
                    stats.score = stats.score.saturating_add(points);
                    stats.score
                } else {
                    0
                };
                // Resource-store sync will also flush the updated PlayerStats.
                serde_json::json!({ "score": score })
            },
        )
        .apply(&bridge);

    // One-off without a set:
    // bridge.register_typed(BridgeCommandMeta::new(…), |world, args| { … });
}
```

`bridge.register(name, handler)` remains available for ad-hoc handlers that are not codegen'd.
## TypeScript: subscribe and call

```tsx
import {
  createBevyApp,
  useResource,
  useQuery,
  useBridgeState,
  callNative,
  View,
  Text,
} from "bevy-react";

type Hud = { hp: number; max_hp: number; score: number };
type EnemyRow = { entity: number; hp: number };

function HudOverlay() {
  // Resource store (same channel as register_resource_store)
  const hud = useResource<Hud>("hud", { hp: 0, max_hp: 100, score: 0 });

  // Query store (same channel as register_query_store)
  const enemies = useQuery<EnemyRow[]>("enemies", []);

  // Selector pattern — only re-render when score identity changes
  const score = useBridgeState("hud", { hp: 0, max_hp: 100, score: 0 }, (s) => s.score);

  return (
    <View style={{ position: "absolute", top: 16, left: 16 }}>
      <Text>{`HP ${hud.hp}`}</Text>
      <Text>{`Score ${score}`}</Text>
      <Text>{`Enemies ${enemies.length}`}</Text>
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
| `useQuery(key, initial)` | Alias of `useBridgeState` for query stores |

## Native globals

| Global | Direction |
|---|---|
| `__react_register_bridge_dispatcher(cb)` | JS registers `(channel, value) => void` |
| `__react_register_bridge_call_resolver(cb)` | JS registers `(callId, value) => void` |
| `__react_flush_bridge()` | Host drains dirty channels + call results |
| `__react_call(name, argsJson, callId)` | JS enqueues a Rust handler call |

## TypeScript codegen (`ts-rs`)

Enabled with the plugin feature `bridge-codegen` (pulls in [`ts-rs`](https://docs.rs/ts-rs)). Chose **ts-rs** over specta for this MVP: serde-attribute compatibility is built-in, and we only need type declarations plus a thin command-wrapper emitter (not full RPC function introspection).

### HUD end-to-end

1. Annotate a serializable resource with `Serialize` + `ts_rs::TS` (see `examples/hud/src/bridge_types.rs`).
2. Define commands once with [`BridgeCommandSet`] — each `.command(BridgeCommandMeta::new(…), handler)` pairs TypeScript meta with the Bevy handler.
3. At startup call `commands.apply(&bridge)` (HUD: `apply_hud_bridge`). For codegen, pass the same set via `GeneratedBridgeTs::with_command_set`.
4. Assert freshness in a unit test and commit `packages/bridge-types/src/`. App helpers (`INITIAL_PLAYER_STATS`, `hpRatio`) stay in HUD `hudTypes.ts` and re-export from `bridge-types`.

```bash
# Rewrite generated files, then verify
./scripts/generate-bridge-types.sh

# CI / local: fail if committed output is stale
cargo test --manifest-path examples/hud/Cargo.toml \
  bridge_types::tests::generated_bridge_typescript_is_fresh -- --exact
```

Typed wrappers (`addScore`, `heal`) call `callNative` with the correct Promise result types. Prefer them over stringly `callNative("add_score", …)` in app UI.

### Extending in your app

```rust
use bevy_react::{
    BridgeCommandMeta, BridgeCommandSet, GeneratedBridgeTs, assert_bridge_typescript_fresh,
};

#[derive(Serialize, ts_rs::TS)]
struct MyState { /* … */ }

fn my_bridge_commands() -> BridgeCommandSet {
    BridgeCommandSet::new().command(
        BridgeCommandMeta::new("do_thing", "doThing", "{ id: number }", "{ ok: boolean }"),
        |world, args| { /* … */ serde_json::json!({ "ok": true }) },
    )
}

fn setup_bridge(bridge: Res<ReactBridge>) {
    my_bridge_commands().apply(&bridge);
}

#[test]
fn generated_bridge_typescript_is_fresh() {
    // Shared workspace package — or a path under your app's UI package.
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../packages/bridge-types/src");
    let bundle = GeneratedBridgeTs::new()
        .with_type::<MyState>("MyState.ts")
        .with_command_set("commands.ts", &my_bridge_commands())
        .with_barrel("index.ts", &["MyState.ts", "commands.ts"]);
    // UPDATE_BRIDGE_TYPES=1 rewrites; otherwise asserts equality.
    assert_bridge_typescript_fresh(&out, &bundle);
}
```

Enable `bevy_react` with `features = ["bridge-codegen"]` and add `ts-rs` to the app crate. Args/result TypeScript strings in [`BridgeCommandMeta`] are still written by hand (closure signatures are not reflected); the set API only removes the second copy of the command name between `register` and the meta table.

### Shared package (`bridge-types`)

Generated bridge TypeScript lives in the workspace package [`packages/bridge-types`](../packages/bridge-types/). HUD (and future apps) import from there — not from a per-app `ui/src/generated/` copy:

```ts
import type { PlayerStats } from 'bridge-types'
import { addScore, heal, PLAYER_STATS_KEYS } from 'bridge-types'
```

Add `"bridge-types": "workspace:*"` to the UI package (see `examples/hud/ui/package.json`). Codegen still runs from the HUD Rust crate; output dir is `packages/bridge-types/src/`. Additional apps can emit into the same package (namespaced filenames) or their own out dir following the same `assert_bridge_typescript_fresh` pattern.

Shape tests remain: Rust serde keys in `bridge_types` tests, plus the Vitest key contract in `packages/bevy-react/tests/bridge.test.ts`.

## Status

| Area | Status |
|---|---|
| Manual `publish` + dirty flush | Done |
| `register_resource_store` + per-frame sync | Done |
| `register_query_store` + `mark_query_dirty` / each-frame | Done |
| `callNative` return values (Promise) | Done |
| `useResource` / selector `useBridgeState` | Done |
| `useQuery` | Done |
| `ts-rs` codegen + typed command wrappers (HUD) | Done |
| Unified `BridgeCommandSet` / `register_typed` (meta + handler) | Done |
| Shared package types beyond HUD | Done (`bridge-types`) |
## Notes

- Do not break or bypass the existing `ReactEventQueue` path; the bridge is a separate channel for app data, not UI events.
- Publish only when values change if you care about flush volume; every `publish` dirties the channel for the next frame. Resource stores already gate on change detection. Query stores gate on `mark_query_dirty` (or each-frame + JSON equality).
- Handlers must be `Send + Sync`. Capture shared state carefully; prefer reading/writing Bevy resources via `&mut World`.
- `callNative` resolves after the Bevy frame that runs `process_react_bridge_calls` and the subsequent `__react_flush_bridge` — typically one frame of latency.
