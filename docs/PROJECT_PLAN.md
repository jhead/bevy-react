# PROJECT_PLAN

Living roadmap for bevy-react. The priority stack below guides new work. Parallel agents may land slices of open items — check `main` before starting.

**One-liner:** React Native for Bevy — invest in the bridge (state, types, styling authority), not the widget set.

## Shipped foundation

The early epic checklist (render pipeline, styles, events, compositional widgets, runtime/HMR, CI) is largely done. Treat it as historical context, not the backlog.

| Area | What works |
|---|---|
| **Render pipeline** | Create/update/destroy nodes, multi-root containers, root teardown on `ReactRoot` despawn, deep-diff updates, recursive unmount/despawn |
| **Styles** | Layout (flex/grid), shorthands, colors, text/`fontFamily`, opacity/shadows/gradients, images (`objectFit`, nine-slice), `pointerEvents` |
| **Events** | Native event queue (no `eval`), click/press/release, keys/modifiers, wheel/scroll, focus, bubbling, pointer-move/drag |
| **Widgets** | Compositional TS: TextInput, ScrollView, Checkbox, Slider, Select, ProgressBar, Portal; `useInteraction` |
| **Bridge (minimal)** | `ReactBridge` channels + HUD example ([BRIDGE.md](BRIDGE.md)) — stringly, not typed stores |
| **Runtime / loading** | WASM job pump, fetch/console, production bundle + Vite HMR, graceful shutdown |
| **Examples / CI** | demo, menu, forms, HUD; smoke tests; `.github/workflows/ci.yml` |
| **Docs** | Getting started, style props, architecture, bridge, contributing, changelog |

Useful verification notes (still valid):

- Headless Boa host-API smoke: `plugin/tests/boa_smoke.rs` (entity tree + synthesized click). Full React/Vite counter bundle e2e still optional.
- Message-handling tests: destroy-subtree, clear-component-on-update, double-destroy idempotent (`plugin/tests/message_handling.rs`).
- Manual + scripted demo smoke: [DEMO_SMOKE.md](DEMO_SMOKE.md), `scripts/demo-smoke.sh`.
- Bevy pin: **0.17.3** ([BEVY_VERSION.md](BEVY_VERSION.md)).
- Leftovers from the old epics (not roadmap drivers): texture atlas indexing; optional criterion micro-bench; gamepad/`bevy_input_focus` TabGroup; OS clipboard; ScrollView scrollbar polish; crates.io/npm publish.

### Conscious non-takes

No XAML/XML markup, no two-way data binding, no full browser, no IMGUI architecture.

---

## Priority stack (source of truth)

Ranked by DX leverage. Checklist items are new work (`[ ]`). Status reflects a codebase audit — update when slices land.

### 1. Typed game-state bridge — *partial*

Manual `ReactBridge` channels exist; not a typed, subscribable store layer.

- [ ] Rust: register resources / components / queries as subscribable stores
- [ ] Per-frame batched dirty notifications (not ad-hoc publish forever)
- [ ] TS: `useResource` / `useQuery` selector hooks
- [ ] Codegen TS types from Rust (ts-rs / specta)
- [ ] Typed command events back to Rust

### 2. Host-side interaction styling + transitions — *none*

JS keeps `onClick` and friends; visual interaction state moves to the host.

- [ ] `style={{ hover, pressed, focused }}` applied in Rust from Bevy `Interaction`
- [ ] Host-side transitions / tweens for those states

### 3. Bevy 0.17 headless widgets — *partial*

Compositional TS widgets work; host mapping to `bevy_ui_widgets` is the direction (not more JS interaction reimplementation).

- [ ] Feature-gate `experimental_bevy_ui_widgets`
- [ ] Map Button / Slider / Checkbox to host-side `bevy_ui_widgets`
- [ ] Delete TS-reimplemented interaction logic as host widgets land
- [ ] Keep thin React wrappers that compose host behavior

### 4. ECS escape hatch — *stub*

Private `node_id` → `Entity` map exists; not a public API.

- [ ] Stable `ref` → `Entity` handle for consumers
- [ ] `<Node components={[...]}>` with Rust bundles registered by name

### 5. Fail loudly in-game — *partial*

`JsRuntimeError` resource + logs exist; no player-visible overlay or style diagnostics.

- [ ] In-game error overlay
- [ ] Warnings for unsupported style props
- [ ] Source-mapped stacks in the overlay / logs

### 6. Devtools over designer tools — *stub*

`injectIntoDevTools` call only. Skip a visual designer.

- [ ] React DevTools over WebSocket
- [ ] bevy-egui inspector for node ↔ entity mapping
- [ ] Component gallery (examples of supported primitives/styles)

### 7. Binary op protocol — *none* (later)

Fabric-style binary ops. Only after priorities **1–2**; required before calling the stack "production ready".

- [ ] Design / `proto/` schema for batched binary mutations
- [ ] Replace or sit beside today's in-process enum RPC

---

## How to contribute against this plan

1. Prefer open items in **1 → 2 → 3** unless you are extending something already shipping.
2. Note which priority (and checklist item) a PR advances.
3. Parallel work: keep file ownership disjoint; rebase if push races.
4. Verification baselines above stay green — don't regress destroy/update/smoke paths while adding bridge/styling features.
