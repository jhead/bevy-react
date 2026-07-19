# Contributing

Thanks for interest in `bevy-react`. The project is early and moving quickly; please read this before opening PRs.

## Status

This is an active prototype. Please treat the public API as unstable. The roadmap lives in [docs/PROJECT_PLAN.md](docs/PROJECT_PLAN.md) — prefer aligning work with open epics (especially correctness, events, and WASM) over drive-by polish.

## Development setup

### Rust plugin

```bash
cargo build --manifest-path plugin/Cargo.toml
cargo test --manifest-path plugin/Cargo.toml
```

### TypeScript package

```bash
cd packages/bevy-react
pnpm install
pnpm build
```

### Demo (end-to-end)

Terminal 1 — Vite UI:

```bash
cd examples/demo/ui
pnpm install --ignore-scripts
pnpm dev
```

Terminal 2 — Bevy host:

```bash
cargo run --manifest-path examples/demo/Cargo.toml
```

## Repo layout

| Path | Role |
|---|---|
| `plugin/` | Rust crate (`bevy_react`) |
| `packages/bevy-react/` | npm package / reconciler |
| `examples/demo/` | Working Bevy + Vite example |
| `docs/` | Architecture, style props, roadmap |

## Guidelines

- Prefer small, focused changes with a clear epic link when possible.
- Match existing style; don't add drive-by refactors.
- Don't leave dead code behind when replacing behavior.
- Avoid unnecessary fallback / silent-error paths — fail clearly when contracts break.
- Update docs when you change supported style props, public APIs, or architecture.
- Packages are not published yet; do not bump for a crates.io/npm release unless that is the explicit goal.

## Pull requests

1. Describe the problem and the approach.
2. Note which epic (if any) the change advances.
3. Include how you tested (unit test, demo click-through, WASM build, etc.).

## License

By contributing, you agree that your contributions are licensed under the MIT License (see [LICENSE](LICENSE)).
