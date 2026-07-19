//! Named bundle registry — attach gameplay components to React UI entities.
//!
//! React nodes may pass `components={['Glow', 'SoundOnHover']}`. Names are stored
//! on the entity as [`ReactBundleNames`]; [`apply_react_bundles`] invokes the
//! matching appliers registered on [`BundleRegistry`].

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use boa_gc::{Finalize, Trace, empty_trace};

use crate::react::systems::ReactBundleNames;

/// Thread-safe map from React node id → Bevy [`Entity::to_bits`].
///
/// Updated when React nodes are spawned/destroyed so JS can resolve entity handles
/// via `__react_entity_id`.
#[derive(Resource, Clone, Default, Finalize)]
pub struct ReactEntityMap {
    inner: Arc<Mutex<HashMap<u64, u64>>>,
}

unsafe impl Trace for ReactEntityMap {
    empty_trace!();
}

impl ReactEntityMap {
    pub fn insert(&self, node_id: u64, entity: Entity) {
        if let Ok(mut map) = self.inner.lock() {
            map.insert(node_id, entity.to_bits());
        }
    }

    pub fn remove(&self, node_id: u64) {
        if let Ok(mut map) = self.inner.lock() {
            map.remove(&node_id);
        }
    }

    pub fn get(&self, node_id: u64) -> Option<u64> {
        self.inner.lock().ok()?.get(&node_id).copied()
    }

    pub fn remove_entities(&self, entities: &[Entity]) {
        if entities.is_empty() {
            return;
        }
        let bits: Vec<u64> = entities.iter().map(|e| e.to_bits()).collect();
        if let Ok(mut map) = self.inner.lock() {
            map.retain(|_, b| !bits.contains(b));
        }
    }
}

type BundleFn = Arc<dyn Fn(Entity, &mut World) + Send + Sync>;

struct BundleEntry {
    apply: BundleFn,
    remove: Option<BundleFn>,
}

/// Registry of named bundles that React can attach via the `components` prop.
#[derive(Resource, Clone, Default)]
pub struct BundleRegistry {
    entries: Arc<Mutex<HashMap<String, BundleEntry>>>,
}

impl BundleRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a named applier. Called when the name appears in `components`.
    pub fn register(
        &self,
        name: impl Into<String>,
        apply: impl Fn(Entity, &mut World) + Send + Sync + 'static,
    ) -> &Self {
        self.insert_entry(name, Arc::new(apply), None)
    }

    /// Register apply + teardown when the name is removed from `components`.
    pub fn register_with_remove(
        &self,
        name: impl Into<String>,
        apply: impl Fn(Entity, &mut World) + Send + Sync + 'static,
        remove: impl Fn(Entity, &mut World) + Send + Sync + 'static,
    ) -> &Self {
        self.insert_entry(name, Arc::new(apply), Some(Arc::new(remove)))
    }

    fn insert_entry(&self, name: impl Into<String>, apply: BundleFn, remove: Option<BundleFn>) -> &Self {
        if let Ok(mut map) = self.entries.lock() {
            map.insert(name.into(), BundleEntry { apply, remove });
        }
        self
    }

    fn entry(&self, name: &str) -> Option<(BundleFn, Option<BundleFn>)> {
        let map = self.entries.lock().ok()?;
        let e = map.get(name)?;
        Some((Arc::clone(&e.apply), e.remove.as_ref().map(Arc::clone)))
    }
}

/// Tracks which registry names have already been applied (diff target for updates).
#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
struct ReactAppliedBundles(Vec<String>);

/// Parse `components` from React props JSON without touching `NodeProps` / style.rs.
pub fn parse_component_names(props_json: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(props_json) else {
        return Vec::new();
    };
    match value.get("components") {
        Some(serde_json::Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(str::to_owned))
            .collect(),
        Some(serde_json::Value::Null) | None => Vec::new(),
        Some(other) => {
            log::warn!("components prop must be a string array, got {other}");
            Vec::new()
        }
    }
}

/// One-liner hook for render.rs: record entity id + desired bundle names.
pub fn sync_bundle_names(
    commands: &mut Commands,
    entity_map: &ReactEntityMap,
    entity: Entity,
    node_id: u64,
    props_json: &str,
) {
    entity_map.insert(node_id, entity);
    let names = parse_component_names(props_json);
    commands.entity(entity).insert(ReactBundleNames(names));
}

/// Drop a node id from the entity lookup map (destroy path).
pub fn forget_node(entity_map: &ReactEntityMap, node_id: u64) {
    entity_map.remove(node_id);
}

/// Drop lookup entries for despawned entities (clear-container path).
pub fn forget_entities(entity_map: &ReactEntityMap, entities: &[Entity]) {
    entity_map.remove_entities(entities);
}

/// Apply / tear down registered bundles when [`ReactBundleNames`] changes.
pub fn apply_react_bundles(world: &mut World) {
    let mut pending: Vec<(Entity, Vec<String>, Vec<String>)> = Vec::new();

    {
        let mut query = world.query::<(Entity, &ReactBundleNames, Option<&ReactAppliedBundles>)>();
        for (entity, desired, applied) in query.iter(world) {
            let current = applied.map(|a| a.0.clone()).unwrap_or_default();
            if desired.0 != current {
                pending.push((entity, desired.0.clone(), current));
            }
        }
    }

    if pending.is_empty() {
        return;
    }

    let registry = world.resource::<BundleRegistry>().clone();

    for (entity, desired, current) in pending {
        for name in current.iter().filter(|n| !desired.contains(n)) {
            if let Some((_, Some(remove))) = registry.entry(name) {
                remove(entity, world);
            }
        }

        for name in desired.iter().filter(|n| !current.contains(n)) {
            match registry.entry(name) {
                Some((apply, _)) => apply(entity, world),
                None => log::warn!("Unknown React bundle name: {name}"),
            }
        }

        if world.get_entity(entity).is_ok() {
            world.entity_mut(entity).insert(ReactAppliedBundles(desired));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::react::client::ReactClient;
    use crate::react::systems::{
        process_react_messages, ReactContext, ReactMessageReceiver, ReactRoot, ReactRootMap,
    };

    #[derive(Component, Debug, PartialEq, Eq)]
    struct Glow;

    #[derive(Component, Debug, PartialEq, Eq)]
    struct SoundOnHover;

    const ROOT_ID: &str = "bundle-registry-root";

    fn setup_app() -> (App, ReactClient, BundleRegistry) {
        let mut app = App::new();
        let registry = BundleRegistry::new();
        registry.register("Glow", |entity, world| {
            world.entity_mut(entity).insert(Glow);
        });
        registry.register_with_remove(
            "SoundOnHover",
            |entity, world| {
                world.entity_mut(entity).insert(SoundOnHover);
            },
            |entity, world| {
                world.entity_mut(entity).remove::<SoundOnHover>();
            },
        );

        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .insert_resource(registry.clone())
            .init_resource::<ReactRootMap>()
            .init_resource::<ReactEntityMap>()
            .add_systems(
                Update,
                (process_react_messages, ApplyDeferred, apply_react_bundles).chain(),
            );

        let (client, receiver) = ReactClient::new();
        app.insert_resource(ReactMessageReceiver(receiver));

        let root_entity = app
            .world_mut()
            .spawn((
                ReactRoot {
                    id: ROOT_ID.to_string(),
                },
                Node::default(),
                ReactContext {
                    nodes: Default::default(),
                    root: None,
                },
            ))
            .id();

        {
            let mut ctx = app
                .world_mut()
                .get_mut::<ReactContext>(root_entity)
                .expect("ReactContext");
            ctx.root = Some(root_entity);
        }
        app.world_mut()
            .resource_mut::<ReactRootMap>()
            .roots
            .insert(ROOT_ID.to_string(), root_entity);

        (app, client, registry)
    }

    #[test]
    fn parse_component_names_from_props_json() {
        assert_eq!(
            parse_component_names(r#"{"components":["Glow","SoundOnHover"]}"#),
            vec!["Glow".to_string(), "SoundOnHover".to_string()]
        );
        assert!(parse_component_names(r#"{"components":null}"#).is_empty());
        assert!(parse_component_names(r#"{"style":{}}"#).is_empty());
    }

    #[test]
    fn create_with_components_applies_registered_bundles() {
        let (mut app, client, _) = setup_app();

        let node_id = client.create_node(
            ROOT_ID.to_string(),
            "bevy-node".into(),
            r#"{"components":["Glow","SoundOnHover"]}"#.into(),
        );
        client.append_child(ROOT_ID.to_string(), 0, node_id);
        app.update();

        let entity = {
            let root = *app
                .world()
                .resource::<ReactRootMap>()
                .roots
                .get(ROOT_ID)
                .unwrap();
            *app.world()
                .get::<ReactContext>(root)
                .unwrap()
                .nodes
                .get(&node_id)
                .expect("node mapped")
        };

        assert!(app.world().get::<Glow>(entity).is_some());
        assert!(app.world().get::<SoundOnHover>(entity).is_some());
        assert_eq!(
            app.world().get::<ReactBundleNames>(entity).map(|n| &n.0),
            Some(&vec!["Glow".to_string(), "SoundOnHover".to_string()])
        );

        let bits = app
            .world()
            .resource::<ReactEntityMap>()
            .get(node_id)
            .expect("entity map entry");
        assert_eq!(Entity::from_bits(bits), entity);
    }

    #[test]
    fn update_components_adds_and_removes_bundles() {
        let (mut app, client, _) = setup_app();

        let node_id = client.create_node(
            ROOT_ID.to_string(),
            "bevy-node".into(),
            r#"{"components":["Glow","SoundOnHover"]}"#.into(),
        );
        client.append_child(ROOT_ID.to_string(), 0, node_id);
        app.update();

        let entity = {
            let root = *app
                .world()
                .resource::<ReactRootMap>()
                .roots
                .get(ROOT_ID)
                .unwrap();
            *app.world()
                .get::<ReactContext>(root)
                .unwrap()
                .nodes
                .get(&node_id)
                .unwrap()
        };

        client.update_node(
            ROOT_ID.to_string(),
            node_id,
            r#"{"components":["Glow"]}"#.into(),
        );
        app.update();

        assert!(app.world().get::<Glow>(entity).is_some());
        assert!(
            app.world().get::<SoundOnHover>(entity).is_none(),
            "SoundOnHover should be removed via teardown"
        );
    }

    #[test]
    fn destroy_forgets_entity_map_entry() {
        let (mut app, client, _) = setup_app();

        let node_id = client.create_node(
            ROOT_ID.to_string(),
            "bevy-node".into(),
            r#"{"components":["Glow"]}"#.into(),
        );
        app.update();
        assert!(app.world().resource::<ReactEntityMap>().get(node_id).is_some());

        client.destroy_node(ROOT_ID.to_string(), node_id);
        app.update();
        assert!(app.world().resource::<ReactEntityMap>().get(node_id).is_none());
    }
}
