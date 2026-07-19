# HUD Example

![HUD screenshot](../../docs/media/hud.png)

Binds Bevy ECS player stats into React via [`ReactBridge`](../../docs/BRIDGE.md):

- Rust publishes `{ hp, max_hp, score, hp_ratio }` on the `"hud"` channel each change
- React reads with `useBridgeState('hud', …)` and draws an HP bar + score
- Buttons call `add_score` / `heal` through `callNative`

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
