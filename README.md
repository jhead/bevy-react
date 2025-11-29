# bevy-react

Build UI for your [Bevy](https://bevy.org/) app using React. A custom React renderer generates [Bevy UI](https://docs.rs/bevy/latest/bevy/ui/index.html) components natively in your Bevy ECS with bidirectional interactivity (e.g. onClick, events, and native Rust FFI).

## Features

*   **It's just React**: Full support for React features including State, Hooks, Context, functional components, etc.
*   **Built on Bevy UI**: Renders directly to `bevy_ui` components (`Node`, `Text`, `ImageNode`, `Button`).
*   **Hot Reloading**: Supports Vite-based HMR for instant UI updates without recompiling.

## Current Status

WIP, don't use this yet.

## Getting Started

### Install the crate

```bash
cargo add bevy_react
```

### Create a React app

Setup a new React project. This can be done a variety of ways but I find the easiest to be using a Vite template.

```bash
npm create vite@latest

  # Select React
  # Typescript (or Javascript)
```

If starting from a template, I recommend deleting the html, public, assets/svgs, etc. as they won't be used.

Setup a minimal `main.tsx`for Bevy:

```jsx
import { createBevyApp } from 'bevy-react';

// Bevy UI components
import { Node } from 'bevy-react';

// A simple React function component
function App() {
    const [count, setCount] = useState(0);

    return (
        <Node>
            <Text>Count: {count}</Text>
            <Button onClick={() => setCount(count + 1)}>
                Increment
            </Button>
        </Node>
    );
}

// Required! Default exporting using this function is how Bevy hooks in.
export default createBevyApp(<App />);
```

### Install the Bevy Plugin

Initialize the plugin in your Bevy app:

```rust
use bevy::prelude::*;
use bevy_react::{ReactBundle, ViteDevSource, ReactPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(ReactPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    // Use Vite dev server to load the React app with hot reloading support
    let js_source_dev = ViteDevSource::default()
        .with_entry_point("src/main.tsx")
        .into();

    // You can also point to a built JS bundle
    // This is useful for toggling between dev and prod modes
    let js_source_prod = ReactScriptSource::from_path("my-module", "my/react/app/dist/bundle.mjs");

    // Spawn the React UI bundle covering the right half of the screen
    commands.spawn(ReactBundle::new(
        // This is the root Node, where the UI will be mounted
        Node {
            width: Val::Percent(50.0),
            height: Val::Percent(100.0),
            left: Val::Percent(50.0),
            top: Val::Percent(0.0),
            position_type: PositionType::Absolute,
            ..default()
        },
        js_source_dev,
    ));
}

```

## How it works

`bevy-react` consists of two parts:

1.  **Host (Rust)**: A Bevy plugin that embeds the [Boa](https://github.com/boa-dev/boa) JavaScript engine on a dedicated worker thread. It exposes a channel-based protocol for communicating with the UI and JS runtime.
2.  **Client (JS/TS)**: A custom [React Reconciler](https://www.npmjs.com/package/react-reconciler) that translates React Virtual DOM operations into native function calls which are sent back to the Rust host.

For instance, React `render()` will call `createInstance()` on the reconciler:

```typescript
createInstance = (
    type: Type,
    props: Props,
    rootContainer: Container,
    hostContext: HostContext
): Instance => {
    // ...
    __react_create_node(/* type and props */);
    // ...
}
```

`__react_create_node` calls into Rust, eventually spawning a Bevy UI Node:

```rust
commands.spawn((style, ReactNode { node_id }))
```

In between, `bevy-react` handles the complexity of:
- RPCs between JS and Rust runtimes, serialization, etc.
- Converting CSS-like properties to Bevy Style objects
- Forwarding `Interaction` and keyboard input to the UI
