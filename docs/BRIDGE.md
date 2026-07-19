# Rust ↔ React data bridge

Minimal public API for pushing ECS/game state into React and calling registered Rust functions from JS. Enough for a HUD binding sketch; not a full RPC framework.

## Overview

```
Bevy system                     React (Boa)
─────────────                   ───────────
ReactBridge::publish("hud", …) ──flush──► useBridgeState("hud")
ReactBridge::register("fn", …) ◄──call── callNative("fn", args)
```

State delivery reuses the same host→JS flush pattern as the event queue: Bevy marks channels dirty, then `flush_react_bridge` runs `__react_flush_bridge()` on the JS thread. JS→Rust calls enqueue on the JS thread and run on the Bevy main thread with `&mut World`.

## Rust: push state

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

## Rust: register JS-callable functions

Handlers run inside `process_react_bridge_calls` (exclusive `World` access):

```rust
fn setup_bridge(bridge: Res<ReactBridge>) {
    bridge.register("add_score", |world, args| {
        let points = args.as_i64().unwrap_or(0) as u32;
        if let Some(mut score) = world.get_resource_mut::<Score>() {
            score.0 += points;
        }
        // Optional: push updated HUD back
        if let Some(score) = world.get_resource::<Score>() {
            world.resource::<ReactBridge>().publish(
                "hud",
                serde_json::json!({ "score": score.0 }),
            );
        }
        serde_json::Value::Null
    });
}

#[derive(Resource)]
struct Score(u32);
```

Return values are currently discarded (fire-and-forget from JS). Prefer `publish` to push results into React.

## TypeScript: subscribe and call

```tsx
import {
  createBevyApp,
  useBridgeState,
  callNative,
  View,
  Text,
} from "bevy-react";

type Hud = { hp: number; score: number };

function HudOverlay() {
  const hud = useBridgeState<Hud>("hud", { hp: 0, score: 0 });

  return (
    <View style={{ position: "absolute", top: 16, left: 16 }}>
      <Text>{`HP ${hud.hp}`}</Text>
      <Text>{`Score ${hud.score}`}</Text>
      <View
        onClick={() => callNative("add_score", 10)}
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
| `installBridgeDispatcher()` | Wire host flush → channel store (called from `createBevyApp`) |
| `subscribeBridge(channel, listener)` | Imperative subscription |
| `getBridgeState(channel)` | Latest snapshot |
| `callNative(name, args?)` | Enqueue `__react_call` |
| `useBridgeState(channel, initial)` | React hook via `useSyncExternalStore` |

## Native globals

| Global | Direction |
|---|---|
| `__react_register_bridge_dispatcher(cb)` | JS registers `(channel, value) => void` |
| `__react_flush_bridge()` | Host drains dirty channels |
| `__react_call(name, argsJson)` | JS enqueues a Rust handler call |

## Notes

- Do not break or bypass the existing `ReactEventQueue` path; the bridge is a separate channel for app data, not UI events.
- Publish only when values change if you care about flush volume; every `publish` dirties the channel for the next frame.
- Handlers must be `Send + Sync`. Capture shared state carefully; prefer reading/writing Bevy resources via `&mut World`.
