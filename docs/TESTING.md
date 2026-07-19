# Testing

How to run the automated checks for this repo (Epic 7 scaffolding).

## TypeScript (`packages/bevy-react`)

Uses [Vitest](https://vitest.dev/) with mocked `__react_*` globals (see `tests/mockReactGlobals.ts`).

```bash
cd packages/bevy-react
pnpm install   # from repo root also works via the workspace
pnpm test      # vitest run
pnpm test:watch
pnpm build     # tsc → dist/
```

Reconciler tests live under `packages/bevy-react/tests/` (outside `src/` so they are not emitted by `tsc`).

## Rust (`plugin`)

```bash
# Native unit / integration tests
cargo test --manifest-path plugin/Cargo.toml

# Lint (also run in CI)
cargo clippy --manifest-path plugin/Cargo.toml --all-targets -- -D warnings

# WASM compile check (fetch feature only; websocket is native-only)
rustup target add wasm32-unknown-unknown
cargo build --manifest-path plugin/Cargo.toml \
  --target wasm32-unknown-unknown \
  --no-default-features --features fetch
```

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
| `rust-wasm` | `cargo build --target wasm32-unknown-unknown` (fetch feature) |
| `bevy-react-ts` | `pnpm build` + `pnpm test` for `packages/bevy-react` |
| `demo-ui` | `pnpm build` for `examples/demo/ui` |

## Not covered yet

- Headless Bevy + Boa end-to-end smoke test (Epic 7 follow-up)
- Shared TS/Rust style-schema sync test (Epic 1)
- Full style.rs / message-handling Rust coverage (owned by other Epic work)
