//! Epic A verification: drive `ReactClientProto` through a headless Bevy app.

use bevy::prelude::*;

use crate::react::client::ReactClient;
use crate::react::systems::{
    process_react_messages, ReactContext, ReactMessageReceiver, ReactRoot, ReactRootMap,
};

const ROOT_ID: &str = "epic-a-root";

fn setup_app() -> (App, ReactClient) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_resource::<ReactRootMap>()
        .init_resource::<crate::react::ReactEntityMap>()
        .add_systems(Update, process_react_messages);

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

    (app, client)
}

fn flush(app: &mut App) {
    app.update();
}

fn root_entity(app: &App) -> Entity {
    *app.world()
        .resource::<ReactRootMap>()
        .roots
        .get(ROOT_ID)
        .expect("root mapped")
}

fn context_nodes(app: &App) -> bevy::platform::collections::HashMap<u64, Entity> {
    app.world()
        .get::<ReactContext>(root_entity(app))
        .expect("ReactContext")
        .nodes
        .clone()
}

fn entity_alive(app: &App, entity: Entity) -> bool {
    app.world().get_entity(entity).is_ok()
}

#[test]
fn destroy_subtree_removes_parent_and_child() {
    let (mut app, client) = setup_app();

    let parent_id = client.create_node(
        ROOT_ID.to_string(),
        "bevy-node".into(),
        r#"{"style":{"width":100}}"#.into(),
    );
    let child_id = client.create_node(
        ROOT_ID.to_string(),
        "bevy-node".into(),
        r#"{"style":{"width":50}}"#.into(),
    );
    client.append_child(ROOT_ID.to_string(), parent_id, child_id);
    client.append_child(ROOT_ID.to_string(), 0, parent_id);
    flush(&mut app);

    let nodes = context_nodes(&app);
    let parent_entity = *nodes.get(&parent_id).expect("parent mapped");
    let child_entity = *nodes.get(&child_id).expect("child mapped");
    assert!(entity_alive(&app, parent_entity));
    assert!(entity_alive(&app, child_entity));

    client.destroy_node(ROOT_ID.to_string(), parent_id);
    flush(&mut app);

    let nodes = context_nodes(&app);
    assert!(
        !nodes.contains_key(&parent_id),
        "parent should be purged from ReactContext"
    );
    assert!(
        !nodes.contains_key(&child_id),
        "child should be purged from ReactContext with parent subtree"
    );
    assert!(
        !entity_alive(&app, parent_entity),
        "parent entity should be despawned"
    );
    assert!(
        !entity_alive(&app, child_entity),
        "child entity should be despawned with subtree"
    );
}

#[test]
fn update_clears_background_color_component() {
    let (mut app, client) = setup_app();

    let node_id = client.create_node(
        ROOT_ID.to_string(),
        "bevy-node".into(),
        r##"{"style":{"backgroundColor":"#ff0000","width":40}}"##.into(),
    );
    client.append_child(ROOT_ID.to_string(), 0, node_id);
    flush(&mut app);

    let entity = *context_nodes(&app).get(&node_id).expect("node mapped");
    assert!(
        app.world().get::<BackgroundColor>(entity).is_some(),
        "BackgroundColor should be present after create with backgroundColor"
    );

    // Style update without backgroundColor → component removed
    client.update_node(
        ROOT_ID.to_string(),
        node_id,
        r#"{"style":{"width":40}}"#.into(),
    );
    flush(&mut app);

    assert!(
        app.world().get::<BackgroundColor>(entity).is_none(),
        "BackgroundColor should be removed when prop is cleared"
    );
    assert!(
        entity_alive(&app, entity),
        "entity should still exist after style clear"
    );
    assert!(
        context_nodes(&app).contains_key(&node_id),
        "node should remain in ReactContext"
    );
}

#[test]
fn double_destroy_is_idempotent() {
    let (mut app, client) = setup_app();

    let node_id = client.create_node(
        ROOT_ID.to_string(),
        "bevy-node".into(),
        r#"{"style":{"width":10}}"#.into(),
    );
    client.append_child(ROOT_ID.to_string(), 0, node_id);
    flush(&mut app);

    let entity = *context_nodes(&app).get(&node_id).expect("node mapped");

    // removeChild + destroy (reconciler), then a second destroy (detachDeletedInstance)
    client.remove_child(ROOT_ID.to_string(), 0, node_id);
    client.destroy_node(ROOT_ID.to_string(), node_id);
    client.destroy_node(ROOT_ID.to_string(), node_id);
    flush(&mut app);

    assert!(!context_nodes(&app).contains_key(&node_id));
    assert!(!entity_alive(&app, entity));

    // Extra destroy after flush must also be a no-op (no panic)
    client.destroy_node(ROOT_ID.to_string(), node_id);
    flush(&mut app);
    assert!(!context_nodes(&app).contains_key(&node_id));
}

#[test]
fn destroy_unknown_node_is_noop() {
    let (mut app, client) = setup_app();
    client.destroy_node(ROOT_ID.to_string(), 999_999);
    flush(&mut app);
    assert!(context_nodes(&app).is_empty());
}
