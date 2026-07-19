//! BRRP soak: encode → `commit_binary_ops` → ECS (requires `binary_ops`).
//!
//! Mirrors the TS reconciler hot path: one frame per React commit batch.

#![cfg(feature = "binary_ops")]

use bevy::prelude::*;
use bevy_react::{
    process_react_messages, ReactClient, ReactContext, ReactEntityMap, ReactMessageReceiver,
    ReactNode, ReactRoot, ReactRootMap,
};
use bevy_react::react::proto::{encode_batch, BinaryOp};

const ROOT_ID: &str = "brrp-soak";

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
fn binary_commit_builds_parent_child_tree() {
    let (mut app, client) = setup_app();

    let bytes = encode_batch(
        ROOT_ID,
        &[
            BinaryOp::CreateNode {
                node_id: 1,
                node_type: "bevy-node".into(),
                props_json: r#"{"style":{"width":100}}"#.into(),
            },
            BinaryOp::CreateNode {
                node_id: 2,
                node_type: "bevy-text".into(),
                props_json: r#"{"content":"Hello"}"#.into(),
            },
            BinaryOp::AppendChild {
                parent_id: 1,
                child_id: 2,
            },
            BinaryOp::AppendChild {
                parent_id: 0,
                child_id: 1,
            },
            BinaryOp::Commit,
        ],
    )
    .expect("encode");

    client.commit_binary_ops(&bytes).expect("commit");
    app.update();

    let parent_e = node_entity(&app, 1);
    let child_e = node_entity(&app, 2);
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
fn binary_soak_mount_update_reorder_destroy() {
    let (mut app, client) = setup_app();

    // Mount: parent with A, B
    let mount = encode_batch(
        ROOT_ID,
        &[
            BinaryOp::CreateNode {
                node_id: 1,
                node_type: "bevy-node".into(),
                props_json: "{}".into(),
            },
            BinaryOp::CreateNode {
                node_id: 2,
                node_type: "bevy-text".into(),
                props_json: r#"{"content":"A"}"#.into(),
            },
            BinaryOp::CreateNode {
                node_id: 3,
                node_type: "bevy-text".into(),
                props_json: r#"{"content":"B"}"#.into(),
            },
            BinaryOp::AppendChild {
                parent_id: 1,
                child_id: 2,
            },
            BinaryOp::AppendChild {
                parent_id: 1,
                child_id: 3,
            },
            BinaryOp::AppendChild {
                parent_id: 0,
                child_id: 1,
            },
            BinaryOp::Commit,
        ],
    )
    .unwrap();
    client.commit_binary_ops(&mount).unwrap();
    app.update();

    let parent_e = node_entity(&app, 1);
    let a_e = node_entity(&app, 2);
    let b_e = node_entity(&app, 3);
    assert_eq!(children_of(&app, parent_e), vec![a_e, b_e]);

    // Update text + style
    let update = encode_batch(
        ROOT_ID,
        &[
            BinaryOp::UpdateNode {
                node_id: 2,
                props_json: r#"{"content":"A2"}"#.into(),
            },
            BinaryOp::UpdateNode {
                node_id: 1,
                props_json: r#"{"style":{"width":42}}"#.into(),
            },
            BinaryOp::Commit,
        ],
    )
    .unwrap();
    client.commit_binary_ops(&update).unwrap();
    app.update();

    let layout = app.world().get::<Node>(parent_e).expect("Node");
    assert_eq!(layout.width, Val::Px(42.0));
    let text = app.world().get::<Text>(a_e).expect("Text");
    assert_eq!(text.0.as_str(), "A2");

    // Reorder B before A
    let reorder = encode_batch(
        ROOT_ID,
        &[
            BinaryOp::InsertBefore {
                parent_id: 1,
                child_id: 3,
                before_id: 2,
            },
            BinaryOp::Commit,
        ],
    )
    .unwrap();
    client.commit_binary_ops(&reorder).unwrap();
    app.update();
    app.update(); // deferred insert_before

    assert_eq!(children_of(&app, parent_e), vec![b_e, a_e]);

    // Destroy subtree
    let destroy = encode_batch(
        ROOT_ID,
        &[
            BinaryOp::RemoveChild {
                parent_id: 0,
                child_id: 1,
            },
            BinaryOp::DestroyNode { node_id: 1 },
            BinaryOp::Commit,
        ],
    )
    .unwrap();
    client.commit_binary_ops(&destroy).unwrap();
    app.update();
    app.update(); // deferred despawn

    assert!(app.world().get_entity(parent_e).is_err());
    assert!(app.world().get_entity(a_e).is_err());
    assert!(app.world().get_entity(b_e).is_err());

    let root = *app
        .world()
        .resource::<ReactRootMap>()
        .roots
        .get(ROOT_ID)
        .unwrap();
    let ctx = app.world().get::<ReactContext>(root).unwrap();
    assert!(!ctx.nodes.contains_key(&1));
    assert!(!ctx.nodes.contains_key(&2));
    assert!(!ctx.nodes.contains_key(&3));
}

#[test]
fn binary_then_enum_alloc_does_not_collide() {
    let (mut app, client) = setup_app();

    let bytes = encode_batch(
        ROOT_ID,
        &[
            BinaryOp::CreateNode {
                node_id: 10,
                node_type: "bevy-node".into(),
                props_json: "{}".into(),
            },
            BinaryOp::AppendChild {
                parent_id: 0,
                child_id: 10,
            },
            BinaryOp::Commit,
        ],
    )
    .unwrap();
    client.commit_binary_ops(&bytes).unwrap();
    app.update();

    // Enum path allocates after the JS-side binary id.
    let next = client.create_node(ROOT_ID.to_string(), "bevy-node".into(), "{}".into());
    assert!(next > 10, "enum alloc should advance past binary id, got {next}");
    client.append_child(ROOT_ID.to_string(), 0, next);
    client.complete();
    app.update();

    assert!(app.world().get::<ReactNode>(node_entity(&app, 10)).is_some());
    assert!(app.world().get::<ReactNode>(node_entity(&app, next)).is_some());
}
