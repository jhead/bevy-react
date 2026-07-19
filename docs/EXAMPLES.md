# Examples

## Available

### Demo (`examples/demo/`)

End-to-end Bevy + Vite + TypeScript sample:

- Spawns a `ReactBundle` covering the right half of the window
- Loads the UI from the Vite dev server with HMR
- Exercises basic components and style props

See [examples/demo/README.md](../examples/demo/README.md) and [GETTING_STARTED.md](GETTING_STARTED.md).

## Planned

Additional examples are tracked under Epic 8 in [PROJECT_PLAN.md](PROJECT_PLAN.md). Intended when the render/event foundations settle:

| Example | Goal |
|---|---|
| Menu screen | Full-screen layout, navigation between panels, button hover/press feedback |
| HUD + game state | Bind Bevy ECS / game state into React (once a data bridge exists) |
| Forms / settings | TextInput, toggles, validation-style flows |

These are documentation placeholders only — the example crates are not in the tree yet.
