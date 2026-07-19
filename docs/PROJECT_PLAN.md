# PROJECT_PLAN

Living roadmap for bevy-react. The priority stack below guides new work. Check `main` before starting — large slices of 1–6 landed in parallel.

**One-liner:** React Native for Bevy — invest in the bridge (state, types, styling authority), not the widget set.

## Shipped foundation

The early epic checklist (render pipeline, styles, events, compositional widgets, runtime/HMR, CI) is done. Treat it as historical context.

| Area | What works |
|---|---|
| **Render pipeline** | Create/update/destroy nodes, multi-root containers, root teardown on `ReactRoot` despawn, deep-diff updates, recursive unmount/despawn |
| **Styles** | Layout (flex/grid), shorthands, colors, text/`fontFamily`, opacity/shadows/gradients, images, `pointerEvents`, **host `hover`/`pressed`/`focused`/`checked` + transitions**, unknown-prop warnings |
| **Events** | Native event queue (no `eval`), click/press/release, keys/modifiers, wheel/scroll, focus, bubbling, pointer-move/drag |
| **Widgets** | Thin React wrappers over `bevy_ui_widgets` for Button/Slider/Checkbox; TextInput, ScrollView, Select, ProgressBar, Portal |
| **Bridge** | Resource stores (`register_resource_store`), `useResource` / selectors, Promise `callNative` ([BRIDGE.md](BRIDGE.md)) |
| **ECS escape** | `useEntityRef` + `components={[…]}` named bundles ([ECS_ESCAPE.md](ECS_ESCAPE.md)) |
| **Errors / DX** | In-game error overlay; best-effort source maps ([TROUBLESHOOTING.md](TROUBLESHOOTING.md)) |
| **Devtools** | WS dump on `:8098`, optional egui node↔entity panel, gallery example ([DEVTOOLS.md](DEVTOOLS.md)) |
| **Runtime / loading** | WASM job pump, fetch/console, production bundle + Vite HMR, graceful shutdown |
| **Examples / CI** | demo, menu, forms, HUD, gallery; smoke tests; `.github/workflows/ci.yml` |

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

Ranked by DX leverage. Status updated after the bridge-first parallel landing.

### 1. Typed game-state bridge — *done* (MVP; polish open)

Named resource/query stores, Promise calls, HUD ts-rs codegen, and typed command wrappers ship.

- [x] Rust: register resources as subscribable stores (`register_resource_store`)
- [x] Per-frame batched dirty notifications (store sync in `flush_react_bridge`)
- [x] TS: `useResource` + selector hooks (`useBridgeState` selector)
- [x] TS: `useQuery` for entity/component subscriptions (`register_query_store` / `mark_query_dirty`)
- [x] Codegen TS types from Rust (`bridge-codegen` + ts-rs; HUD `PlayerStats`)
- [x] Typed command return values to JS (`callNative` → Promise)
- [x] Generated typed command wrappers (`BridgeCommandMeta` → HUD `addScore` / `heal`)
- [ ] Auto-derive command meta from `register` closures (manual meta today)
- [ ] Spread codegen beyond HUD / shared package types

### 2. Host-side interaction styling + transitions — *done* (MVP)

JS keeps `onClick` and friends; visual interaction state is host-owned.

- [x] `style={{ hover, pressed, focused }}` applied in Rust from Bevy `Interaction` / focus
- [x] Host-side transitions / tweens for color/opacity fields
- [x] `style.checked` from Bevy UI `Checked`; hover also from picking `Hovered`

### 3. Bevy 0.17 headless widgets — *done* (MVP)

Host owns Slider/Checkbox interaction; Bevy Slider drag remains horizontal-only upstream.

- [x] Feature-gate `experimental_bevy_ui_widgets`
- [x] Map Button / Slider / Checkbox to host-side `bevy_ui_widgets`
- [x] Thin React wrappers that compose host behavior (delete step-button fake slider)
- [x] Button `Activate` → React `click` (keyboard only; pointer still via Interaction to avoid double-fire)
- [x] Vertical slider / thumb layout polish; host styles for `Checked`/`Hovered` widget markers (`style.checked`)

### 4. ECS escape hatch — *done* (MVP)

- [x] Stable `ref` → `Entity` handle (`useEntityRef` / `__react_entity_id` → `Entity::to_bits`)
- [x] `<Node components={[...]}>` with Rust bundles registered by name (`BundleRegistry`)

### 5. Fail loudly in-game — *done* (MVP; Boa limits)

- [x] In-game error overlay (Dismiss / Esc)
- [x] Warnings for unsupported style props
- [x] Source-mapped stacks when frames have `url:line:col` (Vite maps, `sourceMappingURL`, CallFrame positions)
- [ ] Symbolicate bare VM frame names with no path (Boa platform limit)

### 6. Devtools over designer tools — *done* (MVP; not drop-in RDT)

Skip a visual designer. `:8098` is an RDT-shaped inspector bridge, not `npx react-devtools` on `:8097`.

- [x] Debug WebSocket bridge (`:8098` tree + ecs_map + fiber dump)
- [x] RDT-shaped `{ event, payload }` messages (fiber operations snapshot)
- [x] Optional bevy-egui inspector for node ↔ entity mapping (`egui` / `devtools-full` features)
- [x] Component gallery example (`examples/gallery`)
- [ ] Drop-in `react-devtools-core` / Chrome extension on `:8097` (blocked by Boa init order + Int32 ops codec)

### 7. Binary op protocol — *partial* (codec; reconciler not default)

Fabric-style binary ops. Schema + Rust round-trip codec landed; reconciler still uses enum natives by default.

- [x] Design / `plugin/src/react/proto/` schema for batched binary mutations ([PROTO.md](PROTO.md))
- [x] Sit beside today's in-process enum RPC (`binary_ops` → `__react_commit_ops`)
- [ ] TS reconciler encodes commits into BRRP (stub: `packages/bevy-react/src/protocol.ts`)
- [ ] Optional string-table interning (`FLAG_STRING_TABLE`)
- [ ] Make binary the default hot path once TS encoder + soak tests land

---

## How to contribute against this plan

1. Prefer open items under **1** (codegen / `useQuery`) and **5–6** polish unless extending something already shipping.
2. Note which priority (and checklist item) a PR advances.
3. Parallel work: keep file ownership disjoint; rebase if push races.
4. Verification baselines above stay green — don't regress destroy/update/smoke paths while adding bridge/styling features.
