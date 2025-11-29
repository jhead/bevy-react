# Demo Example

This simple React + Vite + Typescript demo shows:
- How to spawn the React UI in Bevy, mount it to a specific area, and specify the JS source to load
- Basic components available to use in React, including styling props
- Vite hot reloading support

The `ui` project was created using `npm create vite@latest` for React + Typescript, with some code/assets removed (SVGs, CSS, etc.).

## How to run it

Start the Vite dev server:

```bash
cd ui
npm install --ignore-scripts
npm run dev
```

In another terminal, run the Bevy app:

```
cargo run
```

You should see the React UI loaded and rendered on the right side of the Bevy window.

At this point, you can edit the `ui` React/Typescript code, save, and it should automatically hot reload in Bevy!
