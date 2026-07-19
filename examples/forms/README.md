# Forms / Settings Example

Settings-style panel exercising TextInput, Checkbox, Slider, and Select.

## How to run

```bash
cd ui
pnpm install --ignore-scripts
pnpm dev
```

In another terminal:

```bash
cargo run --manifest-path examples/forms/Cargo.toml
```

## Production bundle

```bash
cd examples/forms/ui && pnpm build
cargo run --manifest-path examples/forms/Cargo.toml --release
```
