# Bevy version support

`bevy_react` is developed and CI-tested against a **pinned Bevy release**, not a floating major range.

## Current pin

| Component | Version |
|---|---|
| Bevy (Rust) | **0.17.3** (`plugin/Cargo.toml`) |
| React (JS) | **19** (examples / `packages/bevy-react`) |

The crate does **not** yet publish compatibility guarantees across Bevy minors. Treat upgrades as intentional work items (style API, UI inputs, asset loaders, and feature flags shift between Bevy releases).

## Support policy

1. **One supported Bevy version at a time** — the version in `plugin/Cargo.toml` is the source of truth.
2. **Upgrade path** — when Bevy ships a new patch/minor we intend to support, bump the pin, fix compile breaks, and re-run `cargo test --lib --tests` plus the demo/examples.
3. **No multi-version matrix yet** — CI builds one Bevy target (native + wasm with `--no-default-features --features fetch`). A crates.io/npm multi-version matrix lands with publish (Epic 8).
4. **Downstream apps** — path-depend this repo (or a git rev) and match Bevy **0.17.3** until a wider policy is published.

## Tracking matrix

| Bevy | `bevy_react` | Status |
|---|---|---|
| 0.17.3 | 0.1.0 (unpublished) | **Supported** (current pin) |
| 0.17.x other patches | — | Untested; expect minor churn |
| 0.16 / 0.18+ | — | Not supported |

See also [PROJECT_PLAN.md](PROJECT_PLAN.md) Epic 8 and the root [README.md](../README.md).
