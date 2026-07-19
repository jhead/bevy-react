# HUD Example

![HUD screenshot](../../docs/media/hud.png)

Binds Bevy ECS player stats into React via [`ReactBridge`](../../docs/BRIDGE.md):

- Rust registers `PlayerStats` with `register_resource_store("hud")`
- React reads with `useResource('hud', …)` (HP ratio derived on the TS side)
- Buttons call typed wrappers `addScore` / `heal` (generated from Rust command metadata)
- Shared JSON shape: `ui/src/generated/` ← `ts-rs` from `src/bridge_types.rs` (regen: `./scripts/generate-bridge-types.sh`)

## How to run

```bash
cd ui
pnpm install --ignore-scripts
pnpm dev
```

In another terminal:

```bash
cargo run --manifest-path examples/hud/Cargo.toml
```

## Production bundle

```bash
cd examples/hud/ui && pnpm build
cargo run --manifest-path examples/hud/Cargo.toml --release
```
