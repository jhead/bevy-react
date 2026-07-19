# Demo smoke checklist

Manual verification for the native demo (and related examples). Use this after
event/input/render changes. Automated build/RPC smoke lives in
[`scripts/demo-smoke.sh`](../scripts/demo-smoke.sh).

## Prerequisites

- Rust toolchain that can build Bevy 0.17
- `pnpm` (via corepack or system install)
- A display (native window). Headless CI cannot exercise click/hover/focus.

Vite is **not** required forever: debug builds expect a Vite server; release
builds load `ui/dist/app.js` after `pnpm build` in the example UI folder.

## Automated smoke (no window interaction)

From the repo root:

```bash
./scripts/demo-smoke.sh
```

This:

1. Builds `packages/bevy-react`
2. Runs plugin unit + `message_handling` tests (RPC → ECS)
3. Production-builds `examples/demo/ui`
4. `cargo check`s the demo crate
5. Optionally launches the **release** demo for a few seconds and exits
   (proves Vite is not needed once `dist/app.js` exists)

## Manual demo (native, interactive)

### A. Dev path (Vite + HMR)

Terminal 1:

```bash
cd examples/demo/ui
pnpm install --ignore-scripts
pnpm dev
```

Terminal 2:

```bash
cargo run --manifest-path examples/demo/Cargo.toml
```

Confirm the React panel mounts on the right half of the window.

### B. Release path (no Vite)

```bash
cd packages/bevy-react && pnpm build
cd ../../examples/demo/ui && pnpm install --ignore-scripts && pnpm build
cargo run --manifest-path examples/demo/Cargo.toml --release
```

### Checklist (Epic A)

Mark each while exercising the demo UI:

| # | Check | Pass? |
|---|---|---|
| 1 | UI mounts without a blank panel / `JsRuntimeError` spam | |
| 2 | Click a button → handler runs (counter / log / visual change) | |
| 3 | Hover → hover styles or `onHover` / `useInteraction` feedback | |
| 4 | Tab / click focuses a focusable control; Escape or click-outside blurs | |
| 5 | Focused `TextInput` (forms example) accepts typing, arrows, Home/End | |
| 6 | Wheel over a scrollable region moves content (`overflow: scroll`) | |
| 7 | Edit a TSX file under Vite → HMR reloads without restarting Bevy | |
| 8 | Quit the app cleanly (no hang on `AppExit`) | |

Forms-specific: open `examples/forms` and verify TextInput caret, selection,
clipboard (Ctrl/Cmd+C/X/V/A), Checkbox, Slider, Select.

Menu-specific: open `examples/menu` and walk Play / Options / Credits panels.

HUD-specific: open `examples/hud`, confirm HP/score update from ECS, and that
**+10 Score** / **Heal** call Rust via `callNative`.

## WASM (optional)

WASM interactive smoke is optional and environment-dependent. Minimum bar:

```bash
cargo check --manifest-path plugin/Cargo.toml --target wasm32-unknown-unknown --no-default-features --features fetch
```

Full browser launch is out of scope for this checklist unless a hosted demo page exists.

## Related automation

| What | Where |
|---|---|
| RPC → ECS (destroy, clear style, double-destroy) | `plugin/tests/message_handling.rs`, `plugin/src/react/message_tests.rs` |
| Reconciler host config | `packages/bevy-react` vitest |
| Full Boa+React e2e (counter + click) | Still open — see Epic 7 in [PROJECT_PLAN.md](PROJECT_PLAN.md) |
