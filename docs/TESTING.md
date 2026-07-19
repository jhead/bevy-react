# Testing

How to run the automated checks for this repo (Epic 7).

## TypeScript (`packages/bevy-react`)

Uses [Vitest](https://vitest.dev/) with mocked `__react_*` globals (see `tests/mockReactGlobals.ts`).

```bash
cd packages/bevy-react
pnpm install   # from repo root also works via the workspace
pnpm test      # vitest run
pnpm test:watch
pnpm build     # tsc → dist/
```

Reconciler tests live under `packages/bevy-react/tests/` (outside `src/` so they are not emitted by `tsc`). Coverage includes:

- Mount / append / clear-container sequences
- Unmount (`remove_child` + `destroy_node`)
- Prop updates and no-op when unchanged
- Keyed reorder (`append_child` move-to-end) and mid-list `insert_before`
- Subtree destroy on update; list shuffle with duplicate destroy (removeChild + detachDeletedInstance)

Pass a fresh `instanceMap` into `createBevyReconciler` (per-root maps; see `roots.ts`).

## Rust (`plugin`)

```bash
# Native unit / integration tests
cargo test --manifest-path plugin/Cargo.toml

# Message-handling smoke only (public API, headless MinimalPlugins)
cargo test --manifest-path plugin/Cargo.toml --test message_handling

# Lint (also run in CI)
cargo clippy --manifest-path plugin/Cargo.toml --all-targets -- -D warnings

# WASM compile check (fetch feature only; websocket is native-only)
rustup target add wasm32-unknown-unknown
cargo build --manifest-path plugin/Cargo.toml \
  --target wasm32-unknown-unknown \
  --no-default-features --features fetch
```

| Suite | Location | What it covers |
|-------|----------|----------------|
| Message-handling integration | `plugin/tests/message_handling.rs` | Public `ReactClient` → `process_react_messages` → ECS (create/append, update, insert_before, destroy, clear) |
| Epic A unit checks | `plugin/src/react/message_tests.rs` | Destroy subtree, backgroundColor clear, double-destroy idempotent |
| Style unit tests | `plugin/src/react/style.rs` `#[cfg(test)]` | Partial parse/style coverage (Epic 2 owns expansions) |

## Demo UI (`examples/demo/ui`)

```bash
cd examples/demo/ui
pnpm install --ignore-scripts
pnpm build
```

## CI

GitHub Actions workflow: `.github/workflows/ci.yml`

| Job | What it runs |
|-----|----------------|
| `rust-native` | `cargo test` + `cargo clippy` in `plugin/` |
| `rust-wasm` | `cargo build --target wasm32-unknown-unknown --no-default-features --features fetch` |
| `bevy-react-ts` | `pnpm build` + `pnpm test` for `packages/bevy-react` |
| `demo-ui` | `pnpm build` for `examples/demo/ui` |

## Not covered yet

- Headless Bevy + Boa + React counter end-to-end (RPC→ECS smoke exists; full JS path still open)
- Shared TS/Rust style-schema sync test (Epic 1)
- Full `style.rs` coverage (Epic 2)
- Hot-path benchmarks (Epic 7)
