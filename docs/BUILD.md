# Building a production JS bundle

`bevy-react` runs your UI inside Boa's ESM loader — **not** a browser. Production builds should emit a **single ESM file** with React and your app inlined (no DOM/`document` assumptions).

## Vite template

```ts
// vite.config.ts
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "node:path";

export default defineConfig(({ mode }) => ({
  plugins: [react()],
  resolve: {
    // One React copy — required for hooks / reconciler
    dedupe: ["react"],
    alias: {
      "bevy-react": path.resolve(__dirname, "../../packages/bevy-react/src"),
    },
  },
  define: {
    "process.env.NODE_ENV": JSON.stringify(
      mode === "production" ? "production" : "development",
    ),
  },
  build: {
    // Single ESM artifact for Bevy AssetServer / include_str!
    lib: {
      entry: path.resolve(__dirname, "src/main.tsx"),
      formats: ["es"],
      fileName: () => "app.js",
    },
    rollupOptions: {
      // Bundle react + bevy-react into the output (Boa has no node_modules)
      external: [],
    },
    target: "esnext",
    minify: mode === "production",
    sourcemap: true,
    outDir: "dist",
    emptyOutDir: true,
  },
}));
```

Entry must default-export a `createBevyApp(...)` result:

```tsx
import { createBevyApp, Node, Text } from "bevy-react";
import { App } from "./App";

export default createBevyApp(<App />);
```

Build:

```bash
pnpm build
# → dist/app.js
```

Copy or point Bevy at that file:

- **AssetServer:** put `app.js` under your Bevy `assets/` folder (e.g. `assets/ui/app.js`)
- **Embed:** `include_str!("../assets/ui/app.js")` via `EmbeddedBundleSource`

## Rust loading APIs

### Dev (Vite HMR)

```rust
commands.spawn(
    ViteDevSource::default()
        .with_entry_point("src/main.tsx")
        .into_bundle(Node::default()),
);
```

`into_bundle` adds `ReactHmrRoot` so Vite WebSocket `update` / `full-reload` messages re-set `ReactDirtyFlag` and re-execute the module.

### Release — embedded

```rust
use bevy_react::{EmbeddedBundleSource, ReactBundle, ReactScriptSource};

let source = ReactScriptSource::auto(
    ViteDevSource::default().with_entry_point("src/main.tsx"),
    EmbeddedBundleSource::new("my-app", include_str!("../assets/ui/app.js")),
);

commands.spawn(ReactBundle::new(Node::default(), source));
```

### Release — AssetServer (WASM / packed assets)

```rust
use bevy_react::ReactAssetBundle;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(ReactAssetBundle::new(
        Node::default(),
        &asset_server,
        "ui/app.js",      // under assets/
        "my-app",
    ));
}
```

### Fallible disk path (lazy)

```rust
ReactScriptSource::auto_with(
    || ViteDevSource::default().with_entry_point("src/main.tsx").into(),
    || ReactScriptSource::from_path("assets/ui/app.js").expect("run pnpm build first"),
)
```

`auto` / `auto_with` select Vite when `debug_assertions` are on, otherwise the production source. `NODE_ENV` is set to `production` in release shims (Vite bootstrap forces `development` under HMR).

## Checklist

- [ ] Default export is `createBevyApp(...)`
- [ ] No `react-dom`, no CSS/`index.html` entry for the Bevy host bundle
- [ ] React is bundled (not externalized)
- [ ] Output is one `.js` / `.mjs` ESM file
- [ ] Host loads via `EmbeddedBundleSource`, `ReactAssetBundle`, or `ReactScriptSource::from_path`
