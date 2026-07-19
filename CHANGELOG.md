# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Packages are **not published** to crates.io or npm yet. Version numbers in
`plugin/Cargo.toml` and `packages/bevy-react/package.json` track local development.

## [Unreleased]

### Added

- Bevy plugin (`bevy_react`) embedding Boa and mounting React trees into Bevy UI entities.
- TypeScript package (`bevy-react`) with a custom `react-reconciler` host and intrinsic components (`Node`, `Button`, `Text`, `Image`, `TextInput`).
- RPC bridge for create/append/insert-before/remove/update/destroy/clear operations.
- CSS-like style conversion into Bevy `Node` / colors / border radius / z-index.
- Pointer interaction forwarding (`onClick`, hover enter/leave) and basic keyboard input for focused nodes.
- Vite-based HMR loading via `ViteDevSource` (native, gated by `websocket` / `fetch` features).
- Experimental WASM build path for the JS engine (async job flushing is still limited).
- Demo example under `examples/demo/`.

### Known limitations

See [docs/PROJECT_PLAN.md](docs/PROJECT_PLAN.md). Highlights: entity cleanup on unmount, style prop contract drift (e.g. TS `borderWidth` vs Rust `border`), eval-based event dispatch, incomplete WASM async, and no public Rust↔React game-state bridge yet.
