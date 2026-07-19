# Component Gallery

Poor-man's Storybook: buttons, text, slider, checkbox, inputs, and layout samples.

With the plugin `devtools` feature enabled (default for this example), the host
exposes a debug WebSocket on `ws://127.0.0.1:8098`. See [docs/DEVTOOLS.md](../../docs/DEVTOOLS.md).

## How to run

```bash
cd ui
pnpm install --ignore-scripts
pnpm dev
```

In another terminal:

```bash
cargo run --manifest-path examples/gallery/Cargo.toml
```

## Production bundle

```bash
cd examples/gallery/ui && pnpm build
cargo run --manifest-path examples/gallery/Cargo.toml --release
```

## Inspector (optional)

```bash
cargo run --manifest-path examples/gallery/Cargo.toml --features egui
```
