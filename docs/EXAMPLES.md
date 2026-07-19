# Examples

## Available

### Demo (`examples/demo/`)

End-to-end Bevy + Vite + TypeScript sample:

- Spawns a `ReactBundle` covering the right half of the window
- Loads the UI from the Vite dev server with HMR
- Exercises basic components and style props

See [examples/demo/README.md](../examples/demo/README.md) and [GETTING_STARTED.md](GETTING_STARTED.md).

### Menu (`examples/menu/`)

Full-screen menu with panel navigation (Play / Options / Credits) and hover/press feedback via `useInteraction`.

```bash
cd examples/menu/ui && pnpm install --ignore-scripts && pnpm dev
cargo run --manifest-path examples/menu/Cargo.toml
```

See [examples/menu/README.md](../examples/menu/README.md).

### Forms / settings (`examples/forms/`)

Settings panel exercising `TextInput` (caret/selection/clipboard), `Checkbox`, `Slider`, and `Select`, plus light client-side validation.

```bash
cd examples/forms/ui && pnpm install --ignore-scripts && pnpm dev
cargo run --manifest-path examples/forms/Cargo.toml
```

See [examples/forms/README.md](../examples/forms/README.md).

### HUD (`examples/hud/`)

Game-state binding via [`ReactBridge`](BRIDGE.md): Rust ticks HP/score and publishes on a channel; React reads with `useBridgeState` and can call `add_score` through `callNative`.

```bash
cd examples/hud/ui && pnpm install --ignore-scripts && pnpm dev
cargo run --manifest-path examples/hud/Cargo.toml
```

See [examples/hud/README.md](../examples/hud/README.md).

## Planned

Additional examples are tracked under Epic 8 in [PROJECT_PLAN.md](PROJECT_PLAN.md).
