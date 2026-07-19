I'll explore the codebase structure and key files, then assess completeness.

I've explored the full stack — Rust plugin (JS engine, RPC, render/input systems, style conversion), the TS reconciler package, and the demo. Here's my assessment and roadmap.

## Assessment

The core architecture is sound and works end-to-end: Boa runs React on a worker thread (native) or main thread (WASM), a custom reconciler ships mutations over an RPC enum, and Bevy systems materialize entities. But it's an early prototype (README says "WIP, don't use this yet"), and that's accurate. The biggest structural gaps: entities leak on unmount (`removeChild` detaches but never despawns, and `__react_destroy_node` is never called by the reconciler), the TS/Rust style contracts have drifted (`borderWidth` in TS is silently ignored by Rust, which expects `border`), events are dispatched by string-formatting JS into `eval` (slow and injection-prone), WASM can't run any async JS (`flush_jobs` is a no-op, so promises/timers never resolve — yet event dispatch itself uses `async import()`), multiple roots are plumbed through Rust but broken by a single global `fiberRoot` in TS, and there are essentially no tests (2 unit tests total).

## Epic 1: Correctness of the render pipeline

- Fix entity leak: call `__react_destroy_node` from `removeChild`/`detachDeletedInstance` and despawn descendants recursively.
- Fix `handle_update_node` for text nodes: update `TextColor`/`TextFont` on update (currently only applied at create) and stop inserting layout `Node`/`BackgroundColor` onto `Text` entities.
- Remove stale components on update: clearing `backgroundColor`/`borderColor`/`zIndex`/`borderRadius` from props should remove the component, not leave the old value.
- Reconcile TS `BevyStyle` with Rust `StyleProps` (e.g. `borderWidth`), and add a shared-schema test that keeps them in sync.
- Make `prepareUpdate` send diffs (or at least deep-compare `style`) instead of resending full props JSON on any change.
- Support multiple simultaneous roots: per-root fiber containers in TS (drop the global `fiberRoot`) and per-root instance maps.
- Handle root teardown: despawning a `ReactRoot` entity should unmount the React tree and clean `ReactRootMap`/JS state.
- Replace `unwrap()`/`expect()` in engine and message paths (~16 sites) with logged error recovery so bad JS can't kill the thread.

## Epic 2: Style system completeness

- Add missing layout props: `aspectRatio`, per-axis `overflowX/Y`, `overflowClipMargin`, and full CSS Grid props (`gridTemplateColumns/Rows`, `gridRow/Column`, etc. — `display: grid` parses but is unusable).
- Support 4-value/2-value shorthands for `margin`/`padding`/`borderRadius` ("8px 16px").
- Extend `parse_color`: full CSS named-color table, `hsl()/hsla()`, and shorthand `rgb(0 0 0 / 0.5)` syntax.
- Per-corner border radius and per-side border colors.
- Text styling: `fontFamily` via Bevy font assets, `textAlign`/`JustifyText`, `lineHeight`, line-break behavior, and text shadow.
- Add `opacity`, `boxShadow`, and `BackgroundGradient` support (Bevy 0.17 has these).
- Image props: `objectFit` (ImageNode scale modes), tint color, atlas/slice support.

## Epic 3: Event system

- Replace `eval`-based dispatch with a native event queue: a registered JS callback or per-root dispatch function invoked with structured data (no string interpolation of user-controlled module names).
- Dispatch distinct `onPress`/`onRelease` (declared in types, never fired) plus click with cursor position data.
- Add keyup, key modifiers (shift/ctrl/alt), and use Bevy's logical `Key` (layout-aware text) instead of `Debug`-formatted `KeyCode` — this deletes TextInput's hardcoded US-layout `keyCodeToChar`.
- Add wheel/scroll events and wire `overflow: scroll` to `ScrollPosition`.
- Proper focus management: Tab/arrow navigation, click-outside blur, and programmatic focus API.
- Event bubbling/propagation semantics (capture at least `stopPropagation`-style behavior for nested interactives).
- Gamepad/directional UI navigation (game-critical; consider `bevy_input_focus`).

## Epic 4: Built-in components

- Real `TextInput`: cursor position/blinking, selection, clipboard, Home/End/arrow editing, IME-safe text entry via logical keys.
- `ScrollView` with scrollbar rendering and drag.
- `Checkbox`, `Slider`, `Select/Dropdown`, `ProgressBar` primitives.
- Button pressed/hover visual state props (or a `useInteraction` hook).
- A `Portal`/overlay primitive (tooltips, modals) mapping to `GlobalZIndex`/`UiTargetCamera`.

## Epic 5: JS runtime robustness

- Fix WASM async: replace the no-op `flush_jobs` with an incremental job pump (budgeted `run_jobs` per frame or `wasm_bindgen_futures`) so promises, timers, and the current async event dispatch work.
- Finish the timer shims: `shim.rs` defines a timer queue and `schedule_interval` that nothing drains — implement or delete in favor of `boa_runtime` timers.
- Provide `fetch` to app code (not just the module loader) behind the `fetch` feature.
- Forward JS `console.*` and uncaught errors/rejections into Rust `log` with source/stack info.
- Graceful engine shutdown on `AppExit` and panic recovery that restarts the JS thread rather than silently dying.
- Route `render()`/module-load failures to a visible Bevy-side error state, not just a log line.

## Epic 6: Production loading & HMR

- Production path: load a prebuilt JS bundle through Bevy's `AssetServer` (works on WASM and with packed assets), with an `include_str!` embed option.
- Verify/finish the HMR loop: WebSocket update messages must re-trigger module reload and re-render (re-set `ReactDirtyFlag`), not just connect.
- Auto-detect dev vs. release (Vite in debug builds, bundle in release) with one API.
- Set `NODE_ENV=production` and use React production builds outside dev (shim hardcodes `development`).
- Document and template the app-side build (Vite config that outputs a single ESM bundle without DOM assumptions).

## Epic 7: Quality, testing, CI

- Rust unit tests for full `style.rs` coverage and message-handling (headless `App` with `MinimalPlugins` driving `ReactClientProto` sequences).
- TS tests for the reconciler host config (mock `__react_*` globals, assert RPC call sequences for mount/update/reorder/unmount).
- An end-to-end smoke test: headless Bevy + real Boa rendering a counter app, asserting the entity tree and a synthesized click.
- WASM CI build (`cargo build --target wasm32-unknown-unknown`) plus native test + lint + `pnpm build` pipeline.
- Demote the per-message `log::info!` spam to `trace`/`debug`.
- Benchmark and optimize the hot path (per-update JSON round-trip, per-event dynamic `import()`).

## Epic 8: API polish & release

- Publish `bevy_react` to crates.io and `bevy-react` to npm with locked version pairing.
- Write real docs: rustdoc for the public API, a docs site or README rewrite with getting-started, supported style props table, and architecture notes.
- Ship 2–3 more examples (menu screen, HUD with game-state binding, forms/settings panel).
- Rust↔React data bridge: a supported way to push game state into React (context/store fed from ECS) and call registered Rust functions from JS — the README promises "native Rust FFI" but no public API exists.
- Bevy version support policy and a tracking matrix (currently pinned to 0.17.3).
- License/repo hygiene: fix `repository` URL, changelog, contribution guide, and remove the "don't use this yet" once Epics 1–3 land.

Rough priority: Epic 1 and 3 are prerequisites for anything real (leaks and eval-based events), Epic 5's WASM async fix is load-bearing since events already rely on async imports, then 6 → 2 → 4 → 7 → 8.
