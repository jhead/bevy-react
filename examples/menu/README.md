# Menu Example

Full-screen menu screen built with bevy-react: title, panel navigation, and
button hover/press feedback via `useInteraction`.

## How to run

```bash
cd ui
pnpm install --ignore-scripts
pnpm dev
```

In another terminal:

```bash
cargo run --manifest-path examples/menu/Cargo.toml
```

## Production bundle

```bash
cd examples/menu/ui && pnpm build
cargo run --manifest-path examples/menu/Cargo.toml --release
```
