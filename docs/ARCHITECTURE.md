# Architecture

`bevy-react` is a custom React renderer for Bevy UI. It embeds the [Boa](https://github.com/boa-dev/boa) JavaScript engine in a Bevy plugin, runs React inside that engine, and translates reconciler operations into Bevy ECS mutations.

This document summarizes the design described in [CLAUDE.md](../CLAUDE.md) and the gaps tracked in [PROJECT_PLAN.md](PROJECT_PLAN.md).

## Two-runtime bridge

```
┌─────────────────────────────┐         ┌──────────────────────────────┐
│  JS / React (Boa)           │         │  Rust / Bevy                 │
│                             │  RPC    │                              │
│  createBevyApp / render     │ ──────► │  ReactClientProto messages   │
│  react-reconciler host      │         │  process_react_messages      │
│  __react_* native fns       │ ◄────── │  Interaction / keyboard      │
│                             │ events  │  → synthetic JS events       │
└─────────────────────────────┘         └──────────────────────────────┘
```

- **Host (Rust):** `ReactPlugin` plus `js_bevy::JsPlugin`. On native targets the JS engine runs on a dedicated OS thread; on `wasm32` it uses a WASM-compatible async path.
- **Client (TS):** `packages/bevy-react` implements a `react-reconciler` host config. Mutations call globals such as `__react_create_node`, `__react_append_child`, etc., registered by the Rust host (`native_functions.rs`).

Apps default-export `createBevyApp(<App />)`. The host loads the ESM module and calls `mod.default.render(rootId)`.

## RPC protocol

Messages flow JS → Rust via `ReactClientProto` (`plugin/src/react/client.rs`):

| Message | Purpose |
|---|---|
| `CreateNode` / `CreateText` | Allocate a node id and spawn an entity |
| `AppendChild` / `InsertBefore` / `RemoveChild` | Tree structure |
| `UpdateNode` / `UpdateText` | Props / text content |
| `DestroyNode` / `ClearContainer` | Teardown |

Bevy systems in `plugin/src/react/systems/render.rs` apply these to entities tagged with `ReactNode` / `ReactTextNode`, under a root created by `ReactBundle`.

## Style conversion

React `style` objects are JSON-serialized with node props. Rust deserializes `StyleProps` (`style.rs`), builds a Bevy `Node` via `json_to_style`, and applies visual components (`BackgroundColor`, `BorderColor`, `BorderRadius`, `ZIndex`, text color/font) in the render system.

Details and prop tables: [STYLE_PROPS.md](STYLE_PROPS.md).

## Input flow (Bevy → React)

`plugin/src/react/systems/input.rs` observes Bevy `Interaction` (and hover) plus keyboard state for focused nodes, then invokes into the JS engine to fire handlers (`onClick`, hover, key down, etc.). Event delivery is still relatively coarse (see Epic 3 in the project plan — eval-based dispatch, missing press/release, limited focus).

## Script loading & HMR

- **`ViteDevSource`:** Bootstraps Vite's client and the app entry from a dev server. Uses `websocket` / `fetch` Cargo features on native.
- **`ReactScriptSource::from_path` / `from_string`:** Load a prebuilt or inline module without HMR.
- Production asset-pipeline loading (via Bevy `AssetServer`) is planned (Epic 6), not finished.

## WASM notes

Conditional compilation swaps clocks, threading, and tokio features for `wasm32`. Job flushing is currently a no-op on WASM, so promises/timers and async event paths are limited — tracked under Epic 5.

## Key source locations

| Area | Path |
|---|---|
| Bevy plugin entry | `plugin/src/lib.rs` |
| JS engine (native) | `plugin/src/js/engine.rs` |
| JS engine (WASM) | `plugin/src/js/engine_wasm.rs` |
| Native JS functions | `plugin/src/react/native_functions.rs` |
| RPC client | `plugin/src/react/client.rs` |
| Style conversion | `plugin/src/react/style.rs` |
| Render system | `plugin/src/react/systems/render.rs` |
| Input system | `plugin/src/react/systems/input.rs` |
| Vite / HMR bootstrap | `plugin/src/react/vite.rs` |
| TS reconciler | `packages/bevy-react/src/reconciler.ts` |
| TS components | `packages/bevy-react/src/components/` |
| Demo | `examples/demo/` |

## Current limitations (high level)

Accurate for today; details in [PROJECT_PLAN.md](PROJECT_PLAN.md):

- Unmount / destroy paths are incomplete (entity leaks possible).
- Style contracts between TS and Rust have drifted in places.
- Multiple simultaneous React roots are not fully supported on the TS side.
- No public API yet for pushing ECS game state into React or registering Rust callables from JS (despite the long-term goal of native FFI).
- Packages are unpublished; treat as path-dependency only.
