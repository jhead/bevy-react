//! Best-effort Boa ↔ Bevy smoke (Epic 7).
//!
//! Starts a real native Boa engine with [`ReactJsExtension`], drives the
//! reconciler host API via `evaluate`/`execute` (no React / Vite bundle), and
//! asserts the ECS entity tree plus a synthesized click → text update.
//!
//! ## Limits
//! - Does **not** load the TS `bevy-react` package or a full React counter app.
//! - Pointer/focus systems are not exercised; the click is pushed onto
//!   [`ReactEventQueue`] and flushed through `__react_flush_events`.
//! - Relies on a short poll loop for JS-thread scheduling (not a frame clock).

use std::time::{Duration, Instant};

use bevy::prelude::*;
use bevy_react::js::JsEngineBuilder;
use bevy_react::{
    process_react_messages, ReactBridge, ReactClient, ReactClientReceiver, ReactContext,
    ReactEntityMap, ReactEventQueue, ReactJsExtension, ReactMessageReceiver, ReactNode,
    ReactReloadFlag, ReactRoot, ReactRootMap, FLUSH_EVENTS_SCRIPT,
};
use serde_json::json;

const ROOT_ID: &str = "boa-smoke-root";

fn setup_app(receiver: ReactClientReceiver) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .init_asset::<Image>()
        .init_resource::<ReactRootMap>()
        .init_resource::<ReactEntityMap>()
        .insert_resource(ReactMessageReceiver(receiver))
        .add_systems(Update, process_react_messages);

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

    app
}

fn node_entity(app: &App, node_id: u64) -> Option<Entity> {
    let root = *app.world().resource::<ReactRootMap>().roots.get(ROOT_ID)?;
    app.world()
        .get::<ReactContext>(root)?
        .nodes
        .get(&node_id)
        .copied()
}

fn children_of(app: &App, entity: Entity) -> Vec<Entity> {
    app.world()
        .get::<Children>(entity)
        .map(|c| c.iter().collect())
        .unwrap_or_default()
}

fn pump_until(app: &mut App, js: &bevy_react::js::JsEngineClient, mut pred: impl FnMut(&App) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        js.flush_event_loop();
        app.update();
        if pred(app) {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("timed out waiting for Boa smoke condition");
}

#[test]
fn boa_native_functions_build_tree_and_handle_synthesized_click() {
    let (react_client, receiver) = ReactClient::new();
    let event_queue = ReactEventQueue::new();
    let bridge = ReactBridge::new();
    let reload = ReactReloadFlag::new();
    let entity_map = bevy_react::ReactEntityMap::default();

    let engine = JsEngineBuilder::new()
        .with_extension(ReactJsExtension::new(
            react_client,
            event_queue.clone(),
            bridge,
            reload,
            entity_map,
        ))
        .build()
        .expect("build JsEngine");
    let js = engine.start().expect("start JsEngine");

    let mut app = setup_app(receiver);

    // Tiny counter UI via host native functions (not a React bundle).
    js.execute(format!(
        r#"
        (function () {{
            const ROOT = "{ROOT_ID}";
            const panel = __react_create_node(ROOT, "bevy-node", JSON.stringify({{
                style: {{ width: 200, flexDirection: "column" }}
            }}));
            const label = __react_create_text(ROOT, "Count: 0");
            const button = __react_create_node(ROOT, "bevy-button", JSON.stringify({{
                style: {{ width: 100 }}
            }}));
            const btnText = __react_create_text(ROOT, "Inc");
            __react_append_child(ROOT, button, btnText);
            __react_append_child(ROOT, panel, label);
            __react_append_child(ROOT, panel, button);
            __react_append_child(ROOT, 0, panel);

            globalThis.__smoke = {{ label, button, count: 0, clicks: 0 }};
            __react_register_event_dispatcher(function (rootId, nodeId, type, _payload) {{
                if (rootId !== ROOT) return;
                if (type === "click" && nodeId === globalThis.__smoke.button) {{
                    globalThis.__smoke.count += 1;
                    globalThis.__smoke.clicks += 1;
                    __react_update_text(
                        ROOT,
                        globalThis.__smoke.label,
                        "Count: " + globalThis.__smoke.count
                    );
                }}
            }});
        }})();
        "#
    ));

    // Capture node ids written by JS onto the shared ReactClient channel by
    // waiting until the mapped ECS tree exists.
    let mut panel_id = 0u64;
    let mut label_id = 0u64;
    let mut button_id = 0u64;

    pump_until(&mut app, &js, |app| {
        let root = *app
            .world()
            .resource::<ReactRootMap>()
            .roots
            .get(ROOT_ID)
            .unwrap();
        let ctx = app.world().get::<ReactContext>(root).unwrap();
        // Expect panel + label + button + btnText
        ctx.nodes.len() >= 4
    });

    {
        let root = *app
            .world()
            .resource::<ReactRootMap>()
            .roots
            .get(ROOT_ID)
            .unwrap();
        let ctx = app.world().get::<ReactContext>(root).unwrap();
        let root_children = children_of(&app, root);
        assert_eq!(root_children.len(), 1, "panel should be under root");
        let panel_e = root_children[0];
        assert!(app.world().get::<ReactNode>(panel_e).is_some());

        // Recover ids from ReactContext (insertion order is not guaranteed).
        for (&id, &entity) in ctx.nodes.iter() {
            if entity == panel_e {
                panel_id = id;
            }
        }
        let panel_kids = children_of(&app, panel_e);
        assert_eq!(panel_kids.len(), 2, "label + button under panel");

        let label_e = panel_kids[0];
        let button_e = panel_kids[1];
        assert!(app.world().get::<Text>(label_e).is_some());
        assert_eq!(
            app.world().get::<Text>(label_e).unwrap().0.as_str(),
            "Count: 0"
        );
        assert!(app.world().get::<Button>(button_e).is_some());

        for (&id, &entity) in ctx.nodes.iter() {
            if entity == label_e {
                label_id = id;
            }
            if entity == button_e {
                button_id = id;
            }
        }
        assert_ne!(panel_id, 0);
        assert_ne!(label_id, 0);
        assert_ne!(button_id, 0);
        assert_eq!(node_entity(&app, panel_id), Some(panel_e));
    }

    // Synthesize a click on the button and flush into Boa.
    event_queue.push_event(
        ROOT_ID,
        button_id,
        "click",
        json!({ "x": 0.5, "y": 0.5, "normalized": true }),
    );
    js.execute(FLUSH_EVENTS_SCRIPT);

    pump_until(&mut app, &js, |app| {
        let label_e = node_entity(app, label_id).expect("label mapped");
        app.world()
            .get::<Text>(label_e)
            .map(|t| t.0.as_str() == "Count: 1")
            .unwrap_or(false)
    });

    let label_e = node_entity(&app, label_id).unwrap();
    assert_eq!(
        app.world().get::<Text>(label_e).unwrap().0.as_str(),
        "Count: 1"
    );

    js.shutdown();
}
