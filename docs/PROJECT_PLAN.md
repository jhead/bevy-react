# PROJECT_PLAN

Living roadmap for bevy-react. Items marked `[x]` landed on `main` (verify + iterate as needed).

## Assessment

The core architecture is sound and works end-to-end: Boa runs React on a worker thread (native) or main thread (WASM), a custom reconciler ships mutations over an RPC enum, and Bevy systems materialize entities. Early prototype status remains accurate for production use, but several structural blockers are closed: entity destroy on unmount, native structured event dispatch (no eval), WASM budgeted job pump (`ContextGate`, no leak/future-transmute), multi-root TS containers + instance maps, root teardown on `ReactRoot` despawn, borderWidth sync, and basic CI/tests/docs.

Remaining gaps: style coverage still missing text shadow / line-break and image atlas/slice; built-ins incomplete vs production needs; no public Rust↔React data bridge; Epic A manual demo smoke and full Boa+React e2e still open.

## Epic 1: Correctness of the render pipeline

- [x] Fix entity leak: call `__react_destroy_node` from `removeChild`/`detachDeletedInstance` and despawn descendants recursively.
- [x] Fix `handle_update_node` for text nodes: update `TextColor`/`TextFont` on update and stop inserting layout `Node`/`BackgroundColor` onto `Text` entities.
- [x] Remove stale components on update: clearing `backgroundColor`/`borderColor`/`zIndex`/`borderRadius` from props removes the component.
- [x] Reconcile TS `BevyStyle` with Rust `StyleProps` for `borderWidth` (Rust accepts alias). Shared-schema sync test still TODO.
- [x] Deep-compare props/`style` before update RPC (`commitUpdate` diffs; skip when unchanged).
- [x] Support multiple simultaneous roots: per-root fiber containers in TS (`roots.ts`; drop the global `fiberRoot`) and per-root instance maps.
- [x] Handle root teardown: despawning a `ReactRoot` entity should unmount the React tree and clean `ReactRootMap`/JS state.
- [x] Replace `unwrap()`/`expect()` in engine and message paths (~16 sites) with logged error recovery so bad JS can't kill the thread.

## Epic 2: Style system completeness

- [x] Add missing layout props: `aspectRatio`, per-axis `overflowX/Y`, `overflowClipMargin`, and full CSS Grid props (`gridTemplateColumns/Rows`, `gridRow/Column`, etc. — `display: grid` parses but is unusable).
- [x] Support 4-value/2-value shorthands for `margin`/`padding`/`borderRadius` ("8px 16px").
- [x] Extend `parse_color`: full CSS named-color table, `hsl()/hsla()`, and shorthand `rgb(0 0 0 / 0.5)` syntax.
- [x] Per-corner border radius and per-side border colors.
- [x] Text styling: `fontFamily` via Bevy font assets, `textAlign`/`JustifyText`, and `lineHeight` wired in `apply_text_style*`. (Line-break behavior and text shadow still TODO.)
- [x] Add `opacity`, `boxShadow`, and `BackgroundGradient` support (Bevy 0.17 has these). Wired in `apply_visual_style*`; opacity multiplies into colors (no Bevy `UiOpacity`).
- [x] Image props: `objectFit` (ImageNode scale modes) and tint color wired on create/update. (Atlas/slice support still TODO.)

## Epic 3: Event system

- [x] Replace `eval`-based dispatch with a native event queue + registered JS callback (`ReactEventQueue` / `__react_flush_events`).
- [x] Dispatch distinct `onPress`/`onRelease` plus click with cursor position data. (Click fires on release-within-bounds, DOM-style; press/release stay distinct.)
- [x] Add keyup, key modifiers (shift/ctrl/alt), and logical `Key` (TextInput no longer uses US-layout `keyCodeToChar`).
- [x] Add wheel/scroll events and wire `overflow: scroll` to `ScrollPosition` (lazy `ScrollPosition` insert on wheel; HoverMap + ancestor walk).
- [x] Proper focus management: Tab/arrow navigation, click-outside blur, Bevy `RequestReactFocus`/`RequestReactBlur`, and JS `__react_request_focus`/`__react_request_blur`.
- [x] Event bubbling/propagation semantics (`stopPropagation` on bubbled click/press/release/key/wheel/scroll via `parentId` chain).
- [x] Pointer-move / drag: host emits `mousemove` and `drag` (while pressed) with cursor payload for Slider/ScrollView.
- [x] `FocusedNode.root_id` (renamed from `module_name`); multi-window cursor resolved via the UI node's target camera window.
- [ ] Gamepad/directional UI navigation (skipped for now — `bevy_input_focus` needs TabGroup/`InputFocus` integration beyond current `Focusable`/`Button` tab order).

## Epic 4: Built-in components

- [ ] Real `TextInput`: cursor position/blinking, selection, clipboard, Home/End/arrow editing, IME-safe text entry via logical keys. (Logical keys landed; cursor/selection still basic.)
- [x] `ScrollView` primitive (`overflow: scroll`; scrollbar/drag still TODO).
- [x] `Checkbox`, `Slider`, `Select/Dropdown`, `ProgressBar` primitives (compositional; Slider step-only, no track drag).
- [x] `useInteraction` hook for hover/pressed (pressed depends on host press/release — now wired).
- [x] `Portal` overlay primitive (same-tree absolute; not true portal / `GlobalZIndex` yet).

## Epic 5: JS runtime robustness

- [x] Fix WASM async: budgeted `FrameJobExecutor` pumps promises/timers each Bevy frame (no `block_on` hang).
- [x] Finish the timer shims: delete dead `timers`/`schedule_interval` in `shim.rs` (boa_runtime timers already drained).
- [x] Provide `fetch` to app code (not just the module loader) behind the `fetch` feature.
- [x] Forward JS `console.*` and uncaught errors/rejections into Rust `log` with source/stack info.
- [x] Graceful engine shutdown on `AppExit`; native JS thread panic recovery rebuilds the Boa context and re-registers extensions.
- [x] Route `render()`/module-load failures to a visible Bevy-side error state (`JsRuntimeError` resource; also fed by `console.error` / uncaught rejections / script+module failures).

## Epic 6: Production loading & HMR

- [x] Production path: load a prebuilt JS bundle through Bevy's `AssetServer` (works on WASM and with packed assets), with an `include_str!` embed option.
- [x] Verify/finish the HMR loop: WebSocket update messages must re-trigger module reload and re-render (re-set `ReactDirtyFlag`), not just connect.
- [x] Auto-detect dev vs. release (Vite in debug builds, bundle in release) with one API.
- [x] Set `NODE_ENV=production` and use React production builds outside dev (shim hardcodes `development`).
- [x] Document and template the app-side build (Vite config that outputs a single ESM bundle without DOM assumptions).

## Epic 7: Quality, testing, CI

- [ ] Rust unit tests for full `style.rs` coverage (Epic 2 owns `style.rs` body; a few unit tests exist). Message-handling: headless `App` + `MinimalPlugins` driving `ReactClientProto` — see `plugin/tests/message_handling.rs` and `react/message_tests.rs`.
- [x] TS tests for the reconciler host config (mock `__react_*` globals; mount/unmount, update, reorder/`insert_before`, destroy, list shuffle + duplicate destroy).
- [x] Epic A verification: destroy-subtree, backgroundColor clear→remove component, double-destroy idempotent (Rust); list shuffle removeChild+detachDeletedInstance duplicate destroy (TS).
- [ ] An end-to-end smoke test: headless Bevy + real Boa rendering a counter app, asserting the entity tree and a synthesized click. (RPC→ECS smoke in `plugin/tests/message_handling.rs` lands; full Boa+React still open.)
- [x] WASM CI build + native test + lint + `pnpm build`/`pnpm test` pipeline (`.github/workflows/ci.yml`; wasm uses `--no-default-features --features fetch`).
- [x] Demote the per-message `log::info!` spam to `trace`/`debug` in `systems/render.rs`.
- [ ] Benchmark and optimize the hot path (per-update JSON round-trip; eval import path removed for events).

## Epic 8: API polish & release

- [ ] Publish `bevy_react` to crates.io and `bevy-react` to npm with locked version pairing.
- [x] Write real docs: README rewrite, getting-started, supported style props table, architecture notes, CONTRIBUTING, CHANGELOG. (rustdoc still thin.)
- [ ] Ship 2–3 more examples (menu screen, HUD with game-state binding, forms/settings panel). (Planned in `docs/EXAMPLES.md`.)
- [ ] Rust↔React data bridge: a supported way to push game state into React (context/store fed from ECS) and call registered Rust functions from JS.
- [ ] Bevy version support policy and a tracking matrix (currently pinned to 0.17.3).
- [x] License/repo hygiene: changelog, contribution guide; repository URL OK. Keep non-production warning until Epic 2 leftovers, Epic 4 TextInput depth, data bridge, and Epic 7 e2e land.

Rough priority for remaining work: **Epic A** (manual demo smoke) → Epic 2 leftovers (text shadow / line-break, atlas/slice) → deepen Epic 4/7 → data bridge (Epic 8) → publish.

## Verification backlog (Epic A — from mid-stream review)

- [x] `cargo clippy -D warnings`, full `cargo test`, `tsc`, vitest green on every commit
- [x] FrameJobExecutor: eliminate `Box::leak` + future `transmute` via `ContextGate` in `builder.rs` (attach/detach raw pointer; layout-reinterpret `RefCell` only; drop pending jobs if Context address changes)
- [ ] Manual demo smoke (native; wasm if feasible) for click/hover/focus/keyboard via event queue
- [x] Headless `ReactClientProto` tests: destroy-subtree, clear-component-on-update, double-destroy idempotent (`react/message_tests.rs` + `plugin/tests/message_handling.rs`)
- [x] Reorder-heavy list updates: destroy stays silent when both `removeChild` and `detachDeletedInstance` fire (vitest)
