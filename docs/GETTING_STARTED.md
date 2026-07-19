# Getting Started

`bevy-react` is not published to crates.io or npm yet. Use a path / workspace checkout of this repository.

> **Status:** Early prototype — fine for experimenting and contributing, not for production. See [PROJECT_PLAN.md](PROJECT_PLAN.md).

## Prerequisites

- Rust toolchain (edition 2024 / recent stable)
- [pnpm](https://pnpm.io/) (or npm) for the UI packages
- Bevy **0.17.x** (crate pins `0.17.3`)
- React **19**

## Option A: Run the demo (recommended)

The demo is the fastest way to see a working pipeline with Vite HMR.

**1. Start the React UI**

```bash
cd examples/demo/ui
pnpm install --ignore-scripts
pnpm dev
```

Vite should serve on `http://localhost:5173`.

**2. Start the Bevy host**

```bash
cargo run --manifest-path examples/demo/Cargo.toml
```

You should see the React UI on the right half of the Bevy window. Editing files under `examples/demo/ui/src/` and saving should hot-reload.

More notes: [examples/demo/README.md](../examples/demo/README.md).

## Option B: Wire it into your own Bevy app

### 1. Depend on the Rust crate

In your `Cargo.toml`:

```toml
bevy_react = { path = "../bevy-react/plugin" }  # adjust path
```

Or use a git dependency:

```toml
bevy_react = { git = "https://github.com/jhead/bevy-react" }
```

### 2. Add both plugins

`ReactPlugin` needs the JS engine from `JsPlugin`:

```rust
use bevy::prelude::*;
use bevy_react::{ReactBundle, ReactPlugin, ViteDevSource, js_bevy::JsPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(JsPlugin)
        .add_plugins(ReactPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    let js_source = ViteDevSource::default()
        .with_entry_point("src/main.tsx")
        .into();

    commands.spawn(ReactBundle::new(
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        js_source,
    ));
}
```

`ViteDevSource` defaults to `http://localhost:5173`. Override with `.with_dev_server_url(...)` / `.with_module_name(...)` if needed.

For a prebuilt bundle on disk (no HMR):

```rust
let js_source = ReactScriptSource::from_path("path/to/bundle.mjs")?;
```

### 3. Create the React entry

From a Vite React + TypeScript app, strip DOM-only assets and point the default export at `createBevyApp`:

```tsx
import { useState } from "react";
import { createBevyApp, Node, Text, Button } from "bevy-react";

function App() {
  const [count, setCount] = useState(0);

  return (
    <Node style={{ flexDirection: "column", padding: 16, gap: 8 }}>
      <Text style={{ fontSize: 24, color: "white" }}>Count: {count}</Text>
      <Button onClick={() => setCount((c) => c + 1)}>
        <Text>Increment</Text>
      </Button>
    </Node>
  );
}

export default createBevyApp(<App />);
```

Link the local package (from your UI project):

```bash
pnpm add link:../../packages/bevy-react
# or: "bevy-react": "workspace:*" in a monorepo
```

Build the TS package first if needed:

```bash
cd packages/bevy-react && pnpm build
```

### 4. Components available today

| Component | Maps to | Notes |
|---|---|---|
| `Node` | Bevy `Node` | Layout container |
| `Button` | Bevy `Button` | `onClick`, hover enter/leave; `onPress` / `onRelease` declared but not fired yet |
| `Text` | Bevy `Text` | `fontSize`, `color`; `fontFamily` not supported |
| `Image` | Bevy `ImageNode` | Requires `src` |
| `TextInput` | Focusable input host | Early / limited |

Style reference: [STYLE_PROPS.md](STYLE_PROPS.md).

## Cargo features

| Feature | Default | Purpose |
|---|---|---|
| `websocket` | yes | Vite HMR WebSocket (native) |
| `fetch` | yes | HTTP fetch for Vite / module loading |

## Next steps

- [Architecture](ARCHITECTURE.md) — how RPC and systems fit together
- [PROJECT_PLAN.md](PROJECT_PLAN.md) — known gaps and roadmap
- [CONTRIBUTING.md](../CONTRIBUTING.md) — local workflow for contributors
