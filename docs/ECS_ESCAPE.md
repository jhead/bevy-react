# ECS escape hatch

React UI nodes are Bevy entities. This escape hatch lets you attach gameplay
components by name and resolve entity handles from TypeScript.

## Attach named bundles

Register appliers on `BundleRegistry`, then pass names via the `components` prop:

```rust
use bevy::prelude::*;
use bevy_react::{BundleRegistry, ReactPlugin};

#[derive(Component)]
struct Glow;

#[derive(Component)]
struct SoundOnHover;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, ReactPlugin))
        .add_systems(Startup, register_bundles)
        .run();
}

fn register_bundles(registry: Res<BundleRegistry>) {
    registry
        .register("Glow", |entity, world| {
            world.entity_mut(entity).insert(Glow);
        })
        .register_with_remove(
            "SoundOnHover",
            |entity, world| {
                world.entity_mut(entity).insert(SoundOnHover);
            },
            |entity, world| {
                world.entity_mut(entity).remove::<SoundOnHover>();
            },
        );
}
```

```tsx
import { Node, useEntityRef } from "bevy-react";

function HudPip() {
  const [ref, entity] = useEntityRef();

  return (
    <Node
      ref={ref}
      components={["Glow", "SoundOnHover"]}
      style={{ width: 32, height: 32, backgroundColor: "#4ade80" }}
    />
  );
  // entity?.bits → Entity::from_bits(bits) on the Rust side
}
```

Desired names are stored on the entity as `ReactBundleNames`. The
`apply_react_bundles` system diffs against previously applied names and runs
registry apply / remove callbacks.

## Resolve entity handles

After Bevy processes `CreateNode`, JS can look up `Entity::to_bits()`:

```ts
import { resolveEntity, useEntity, useEntityRef } from "bevy-react";

// From a host ref (preferred)
const [ref, entity] = useEntityRef();

// From a known React node id
const handle = resolveEntity(nodeId);
// handle.bits / handle.index / handle.generation

// Hook that polls until the map is populated
const entity = useEntity(nodeId);
```

Native API: `__react_entity_id(nodeId) -> number | null` (packed bits).

In Rust:

```rust
let entity = Entity::from_bits(bits);
```

## Notes

- Node ids are assigned on the JS thread; entity bits appear after the next
  Bevy update that processes the create message.
- Unknown bundle names log a warning and are skipped.
- Prefer `register_with_remove` when clearing `components` should tear down
  gameplay state cleanly.
