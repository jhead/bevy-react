# DevTools

bevy-react ships a **debug direction** (not a visual designer): register the
custom reconciler with React’s DevTools hook, dump the host component tree,
and inspect `node_id` ↔ Bevy `Entity` mapping.

## Feature flags (`plugin/Cargo.toml`)

| Feature | What it enables |
|---|---|
| `devtools` | JSON WebSocket bridge on `ws://127.0.0.1:8098` |
| `egui` | egui “React Nodes” panel over `ReactContext.nodes` |
| `devtools-full` | `devtools` + `egui` |

Default builds do **not** enable either — no forced egui dependency.

```toml
bevy_react = { path = "...", features = ["devtools"] }
# or
bevy_react = { path = "...", features = ["devtools-full"] }
```

`ReactPlugin` auto-registers `ReactDevToolsPlugin` / `ReactNodeInspectorPlugin`
when the matching features are on.

## Connect to the debug bridge

1. Run a host with the `devtools` feature (the [gallery](../examples/gallery/)
   example enables it).
2. Open a WebSocket client to `ws://127.0.0.1:8098`.

Example with websocat:

```bash
websocat ws://127.0.0.1:8098
```

Or from Node:

```js
const ws = new WebSocket('ws://127.0.0.1:8098');
ws.onmessage = (e) => console.log(JSON.parse(e.data));
ws.onopen = () => ws.send(JSON.stringify({ type: 'request_dump' }));
```

### Message types

| `type` | Source | Payload |
|---|---|---|
| `hello` | Rust | Protocol banner |
| `ecs_map` | Rust | Per-root `nodeId` → Entity rows from `ReactContext.nodes` |
| `tree` | JS | Host instance tree from `__bevyReactDevTools.dump()` |
| `request_dump` / `ping` | Client → server | Re-broadcast latest `ecs_map` + `tree` |

The TS package auto-connects to `:8098` when `WebSocket` is available (Boa
shim) and pushes `tree` snapshots about every 2s.

In the JS console / Boa REPL:

```js
__bevyReactDevTools.dump()
```

## `injectIntoDevTools`

`ensureRoot` calls `injectBevyReactDevTools`, which wraps
`reconciler.injectIntoDevTools` so a `__REACT_DEVTOOLS_GLOBAL_HOOK__` (if
present) can see the bevy-react renderer.

## Standalone React DevTools (`npx react-devtools`)

Official standalone DevTools listens on **port 8097** and speaks the full
backend protocol via [`react-devtools-core`](https://www.npmjs.com/package/react-devtools-core)
(`connectToDevTools`). That stack is heavier than Boa’s environment reliably
supports today, so this MVP uses the custom `:8098` bridge instead.

Future work: optional `react-devtools-core` + our WebSocket shim targeting
`:8097` when feasible.

## Entity mapping inspector

```bash
cargo run --manifest-path examples/gallery/Cargo.toml --features egui
```

(with gallery’s `egui` feature forwarding to `bevy_react/egui`), or enable
`bevy_react/egui` / `devtools-full` on your app. An egui window titled
**React Nodes** lists each root’s `node_id` ↔ `Entity` map.
