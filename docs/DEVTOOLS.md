# DevTools

bevy-react ships a **debug direction** (not a visual designer): register the
custom reconciler with React’s DevTools hook, dump host + fiber trees, and
inspect `node_id` ↔ Bevy `Entity` mapping.

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

#### Legacy (`type` field)

| `type` | Source | Payload |
|---|---|---|
| `hello` | Rust | Protocol banner (`bevy-react-devtools-v2`) |
| `ecs_map` | Rust | Per-root `nodeId` → Entity rows from `ReactContext.nodes` |
| `tree` | JS | Host instance tree from `__bevyReactDevTools.dump()` |
| `fiber_tree` | JS | Reconciler fiber walk from `__bevyReactDevTools.dumpFibers()` |
| `request_dump` / `ping` | Client → server | Re-broadcast latest snapshots |

#### RDT-shaped (`event` + `payload`)

These mirror React DevTools bridge **envelope** shapes so tooling can share
parsers. Payloads are JSON (not the Int32 `operations` codec).

| `event` | Meaning |
|---|---|
| `backendVersion` | `bevy-react@0.1.0` |
| `bridgeProtocol` | Advertised protocol + honesty note |
| `rendererAttached` | Renderer metadata (`rendererPackageName: bevy-react`) |
| `operations` | **JSON fiber snapshot** (`kind: "bevy-fiber-snapshot"`), not binary ops |
| `request_dump` / `ping` | Same as legacy dump request |

The TS package auto-connects to `:8098` when `WebSocket` is available (Boa
shim) and pushes snapshots about every 2s.

In the JS console / Boa REPL:

```js
__bevyReactDevTools.dump()
__bevyReactDevTools.dumpFibers()
```

## `injectIntoDevTools`

`ensureRoot` calls `injectBevyReactDevTools`, which wraps
`reconciler.injectIntoDevTools` so a `__REACT_DEVTOOLS_GLOBAL_HOOK__` (if
present) can see the bevy-react renderer.

## Standalone React DevTools (`npx react-devtools`) — remaining gap

Official standalone DevTools listens on **port 8097** and expects
[`react-devtools-core`](https://www.npmjs.com/package/react-devtools-core)
(`initialize` **before** React imports, then `connectToDevTools`) speaking the
full backend protocol (Int32 `operations`, `inspectElement`, profiling, …).

**Why Boa blocks a faithful hookup today**

1. **Init order** — `initialize()` must run before `react` / `react-reconciler`
   load. Vite app bundles already include React; injecting the hook late is a
   no-op for renderer registration timing.
2. **Protocol fidelity** — standalone UI expects binary `operations` patches,
   not a full-tree JSON dump. Emitting correct ops requires the official
   backend agent, not a hand-rolled serializer.
3. **Environment** — `react-devtools-core` assumes browser/RN surfaces (storage
   APIs, richer WebSocket edge cases, etc.) beyond our Boa shims.

**What we ship instead:** the `:8098` bridge with RDT-shaped `{ event, payload }`
messages and a walked fiber tree. Useful for custom inspectors and for evolving
toward full RDT; **not** a drop-in for the Electron DevTools UI or Chrome
extension.

Future: optional pre-bundle hook script + `react-devtools-core` against the
WebSocket shim targeting `:8097` if/when Boa + bundling can guarantee early
`initialize`.

## Entity mapping inspector

```bash
cargo run --manifest-path examples/gallery/Cargo.toml --features egui
```

(with gallery’s `egui` feature forwarding to `bevy_react/egui`), or enable
`bevy_react/egui` / `devtools-full` on your app. An egui window titled
**React Nodes** lists each root’s `node_id` ↔ `Entity` map.
