//! Headless message-handling smoke tests (Epic 7).
//!
//! Drives public `ReactClient` / `ReactClientProto` sequences through
//! `process_react_messages` and asserts the resulting ECS tree.
//! Does not start Boa / JsPlugin (full JS e2e remains a follow-up).

use bevy::prelude::*;
use bevy_react::{
    on_react_root_removed, process_react_messages, FocusedNode, ReactClient, ReactContext,
    ReactEntityMap, ReactMessageReceiver, ReactNode, ReactRoot, ReactRootMap,
};

const ROOT_ID: &str = "test-root";

fn setup_app() -> (App, ReactClient) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .init_asset::<Image>()
        .init_resource::<ReactRootMap>()
        .init_resource::<ReactEntityMap>()
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
                root: None,
                nodes: Default::default(),
            },
        ))
        .id();

    app.world_mut()
        .get_mut::<ReactContext>(root_entity)
        .unwrap()
        .root = Some(root_entity);
    app.world_mut()
        .resource_mut::<ReactRootMap>()
        .roots
        .insert(ROOT_ID.to_string(), root_entity);

    (app, client)
}

fn node_entity(app: &App, node_id: u64) -> Entity {
    let root = *app
        .world()
        .resource::<ReactRootMap>()
        .roots
        .get(ROOT_ID)
        .expect("root mapped");
    *app.world()
        .get::<ReactContext>(root)
        .expect("ReactContext")
        .nodes
        .get(&node_id)
        .unwrap_or_else(|| panic!("missing node_id={node_id}"))
}

fn children_of(app: &App, entity: Entity) -> Vec<Entity> {
    app.world()
        .get::<Children>(entity)
        .map(|c| c.iter().collect())
        .unwrap_or_default()
}

#[test]
fn create_append_builds_parent_child_tree() {
    let (mut app, client) = setup_app();

    let parent = client.create_node(
        ROOT_ID.to_string(),
        "bevy-node".into(),
        r#"{"style":{"width":100}}"#.into(),
    );
    let child = client.create_node(
        ROOT_ID.to_string(),
        "bevy-text".into(),
        r#"{"content":"Hello"}"#.into(),
    );
    client.append_child(ROOT_ID.to_string(), parent, child);
    client.append_child(ROOT_ID.to_string(), 0, parent);
    client.complete();

    app.update();

    let parent_e = node_entity(&app, parent);
    let child_e = node_entity(&app, child);
    let root_e = *app
        .world()
        .resource::<ReactRootMap>()
        .roots
        .get(ROOT_ID)
        .unwrap();

    assert!(app.world().get::<ReactNode>(parent_e).is_some());
    assert!(app.world().get::<ReactNode>(child_e).is_some());
    assert_eq!(children_of(&app, parent_e), vec![child_e]);
    assert_eq!(children_of(&app, root_e), vec![parent_e]);
}

#[test]
fn update_node_applies_style_and_update_text_changes_content() {
    let (mut app, client) = setup_app();

    let node = client.create_node(
        ROOT_ID.to_string(),
        "bevy-node".into(),
        "{\"style\":{\"width\":10,\"backgroundColor\":\"#ff0000\"}}".into(),
    );
    let text = client.create_text(ROOT_ID.to_string(), "before".into());
    client.append_child(ROOT_ID.to_string(), node, text);
    client.append_child(ROOT_ID.to_string(), 0, node);
    app.update();

    client.update_node(
        ROOT_ID.to_string(),
        node,
        "{\"style\":{\"width\":42,\"backgroundColor\":\"#00ff00\"}}".into(),
    );
    client.update_text(ROOT_ID.to_string(), text, "after".into());
    app.update();

    let node_e = node_entity(&app, node);
    let text_e = node_entity(&app, text);

    let layout = app.world().get::<Node>(node_e).expect("Node");
    assert_eq!(layout.width, Val::Px(42.0));
    assert!(app.world().get::<BackgroundColor>(node_e).is_some());

    let content = app.world().get::<Text>(text_e).expect("Text");
    assert_eq!(content.0.as_str(), "after");
}

#[test]
fn insert_before_reorders_siblings() {
    let (mut app, client) = setup_app();

    let parent = client.create_node(
        ROOT_ID.to_string(),
        "bevy-node".into(),
        "{}".into(),
    );
    let a = client.create_node(
        ROOT_ID.to_string(),
        "bevy-text".into(),
        r#"{"content":"A"}"#.into(),
    );
    let b = client.create_node(
        ROOT_ID.to_string(),
        "bevy-text".into(),
        r#"{"content":"B"}"#.into(),
    );
    client.append_child(ROOT_ID.to_string(), parent, a);
    client.append_child(ROOT_ID.to_string(), parent, b);
    client.append_child(ROOT_ID.to_string(), 0, parent);
    app.update();

    let parent_e = node_entity(&app, parent);
    let a_e = node_entity(&app, a);
    let b_e = node_entity(&app, b);
    assert_eq!(children_of(&app, parent_e), vec![a_e, b_e]);

    // Move B before A
    client.insert_before(ROOT_ID.to_string(), parent, b, a);
    app.update();
    // insert_before queues a deferred world command
    app.update();

    assert_eq!(children_of(&app, parent_e), vec![b_e, a_e]);
}

#[test]
fn destroy_node_despawns_subtree_and_clears_context() {
    let (mut app, client) = setup_app();

    let parent = client.create_node(
        ROOT_ID.to_string(),
        "bevy-node".into(),
        "{}".into(),
    );
    let child = client.create_node(
        ROOT_ID.to_string(),
        "bevy-text".into(),
        r#"{"content":"x"}"#.into(),
    );
    client.append_child(ROOT_ID.to_string(), parent, child);
    client.append_child(ROOT_ID.to_string(), 0, parent);
    app.update();

    let parent_e = node_entity(&app, parent);
    let child_e = node_entity(&app, child);

    client.remove_child(ROOT_ID.to_string(), 0, parent);
    client.destroy_node(ROOT_ID.to_string(), parent);
    app.update();
    // destroy queues deferred despawn
    app.update();

    assert!(app.world().get_entity(parent_e).is_err());
    assert!(app.world().get_entity(child_e).is_err());

    let root = *app
        .world()
        .resource::<ReactRootMap>()
        .roots
        .get(ROOT_ID)
        .unwrap();
    let ctx = app.world().get::<ReactContext>(root).unwrap();
    assert!(!ctx.nodes.contains_key(&parent));
    assert!(!ctx.nodes.contains_key(&child));
}

#[test]
fn update_clears_background_color_component() {
    let (mut app, client) = setup_app();

    let node = client.create_node(
        ROOT_ID.to_string(),
        "bevy-node".into(),
        "{\"style\":{\"backgroundColor\":\"#ff0000\",\"width\":40}}".into(),
    );
    client.append_child(ROOT_ID.to_string(), 0, node);
    app.update();

    let entity = node_entity(&app, node);
    assert!(app.world().get::<BackgroundColor>(entity).is_some());

    client.update_node(
        ROOT_ID.to_string(),
        node,
        "{\"style\":{\"width\":40}}".into(),
    );
    app.update();

    assert!(
        app.world().get::<BackgroundColor>(entity).is_none(),
        "BackgroundColor should be removed when prop is cleared"
    );
    assert!(app.world().get_entity(entity).is_ok());
}

#[test]
fn double_destroy_is_idempotent() {
    let (mut app, client) = setup_app();

    let node = client.create_node(
        ROOT_ID.to_string(),
        "bevy-node".into(),
        "{}".into(),
    );
    client.append_child(ROOT_ID.to_string(), 0, node);
    app.update();

    let entity = node_entity(&app, node);

    // removeChild + destroy, then detachDeletedInstance-style second destroy
    client.remove_child(ROOT_ID.to_string(), 0, node);
    client.destroy_node(ROOT_ID.to_string(), node);
    client.destroy_node(ROOT_ID.to_string(), node);
    app.update();
    app.update();

    assert!(app.world().get_entity(entity).is_err());

    // Extra destroy after flush must also be a no-op (no panic)
    client.destroy_node(ROOT_ID.to_string(), node);
    app.update();

    let root = *app
        .world()
        .resource::<ReactRootMap>()
        .roots
        .get(ROOT_ID)
        .unwrap();
    assert!(
        !app.world()
            .get::<ReactContext>(root)
            .unwrap()
            .nodes
            .contains_key(&node)
    );
}

#[test]
fn clear_container_despawns_all_mapped_nodes() {
    let (mut app, client) = setup_app();

    let a = client.create_node(
        ROOT_ID.to_string(),
        "bevy-node".into(),
        "{}".into(),
    );
    let b = client.create_node(
        ROOT_ID.to_string(),
        "bevy-node".into(),
        "{}".into(),
    );
    client.append_child(ROOT_ID.to_string(), 0, a);
    client.append_child(ROOT_ID.to_string(), 0, b);
    app.update();

    let a_e = node_entity(&app, a);
    let b_e = node_entity(&app, b);

    client.clear_container(ROOT_ID.to_string());
    app.update();

    assert!(app.world().get_entity(a_e).is_err());
    assert!(app.world().get_entity(b_e).is_err());

    let root = *app
        .world()
        .resource::<ReactRootMap>()
        .roots
        .get(ROOT_ID)
        .unwrap();
    assert!(app.world().get::<ReactContext>(root).unwrap().nodes.is_empty());
}

#[test]
fn despawning_react_root_clears_map_and_mapped_nodes() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .init_asset::<Image>()
        .init_resource::<ReactRootMap>()
        .init_resource::<ReactEntityMap>()
        .init_resource::<FocusedNode>()
        .add_observer(on_react_root_removed)
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
                root: None,
                nodes: Default::default(),
            },
        ))
        .id();

    app.world_mut()
        .get_mut::<ReactContext>(root_entity)
        .unwrap()
        .root = Some(root_entity);
    app.world_mut()
        .resource_mut::<ReactRootMap>()
        .roots
        .insert(ROOT_ID.to_string(), root_entity);

    let a = client.create_node(ROOT_ID.to_string(), "bevy-node".into(), "{}".into());
    let b = client.create_node(ROOT_ID.to_string(), "bevy-node".into(), "{}".into());
    // Parent one under root; leave the other orphaned (still mapped).
    client.append_child(ROOT_ID.to_string(), 0, a);
    client.complete();
    app.update();

    let a_e = node_entity(&app, a);
    let b_e = node_entity(&app, b);

    app.world_mut().entity_mut(root_entity).despawn();
    app.update();

    assert!(
        app.world()
            .resource::<ReactRootMap>()
            .roots
            .get(ROOT_ID)
            .is_none(),
        "ReactRootMap entry should be removed"
    );
    assert!(
        app.world().get_entity(a_e).is_err(),
        "child node should be despawned"
    );
    assert!(
        app.world().get_entity(b_e).is_err(),
        "orphaned mapped node should be despawned"
    );
}
