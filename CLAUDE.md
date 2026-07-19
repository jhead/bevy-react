# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`bevy-react` is a custom React renderer for Bevy's UI system. It embeds the [Boa](https://github.com/boa-dev/boa) JavaScript engine in a Bevy plugin, runs React on a dedicated worker thread, and translates React Virtual DOM operations into native Bevy ECS entity/component mutations.

## Commands

### Rust (plugin / examples)

```bash
# Build the plugin
cargo build --manifest-path plugin/Cargo.toml

# Run the demo (requires Vite dev server running separately)
cargo run --manifest-path examples/demo/Cargo.toml

# Run tests
cargo test --manifest-path plugin/Cargo.toml
```

### TypeScript (bevy-react package)

```bash
# From packages/bevy-react/
pnpm build      # compile TypeScript → dist/
pnpm dev        # watch mode
```

### Demo UI (Vite + React)

```bash
# From examples/demo/ui/
pnpm install --ignore-scripts
pnpm dev        # starts Vite dev server on localhost:5173
pnpm build      # production bundle
pnpm lint       # ESLint
```

## Architecture

### Two-runtime bridge

The system bridges two runtimes:

- **Rust/Bevy (host)**: `ReactPlugin` spawns a `JsEngine` on a dedicated OS thread (native) or uses WASM-compatible async (wasm32). The engine runs the bundled React app inside Boa.
- **TypeScript (client)**: A custom `react-reconciler` translates React tree operations into calls to global JS functions (`__react_create_node`, `__react_append_child`, etc.) that are registered by the Rust host.

### RPC protocol

`ReactClientProto` (defined in `plugin/src/react/client.rs`) is the message enum flowing from JS → Rust:
- `CreateNode`, `CreateText`, `AppendChild`, `RemoveChild`, `UpdateNode`, `UpdateText`, `DestroyNode`, `ClearContainer`

The Rust side processes these messages in Bevy systems (`plugin/src/react/systems/render.rs`) and spawns/mutates ECS entities tagged with `ReactNode`.

### CSS → Bevy style

`plugin/src/react/style.rs` converts the CSS-like style props passed from React (e.g. `flexDirection`, `width`, `color`) into Bevy `Node` / `BackgroundColor` / `BorderColor` components.

### Input flow (Bevy → React)

`plugin/src/react/systems/input.rs` reads Bevy `Interaction` events and keyboard input, then calls back into the JS engine to fire React synthetic events (onClick, onChange, etc.).

### Hot reloading

`ViteDevSource` (`plugin/src/react/vite.rs`) fetches the entry module from the Vite dev server and uses a WebSocket to receive HMR updates. The `websocket` and `fetch` Cargo features gate this functionality.

### WASM support

Conditional compilation (`cfg(target_arch = "wasm32")`) throughout the plugin replaces:
- `tokio` full → tokio with `sync`+`macros` only
- `StdClock` → `WasmClock` in Boa context
- `flush_jobs` → no-op on wasm32
- Native thread spawning → WASM-compatible async tasks

### Key source locations

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
| TS reconciler | `packages/bevy-react/src/reconciler.ts` |
| TS components | `packages/bevy-react/src/components/` |
| Demo app | `examples/demo/` |
