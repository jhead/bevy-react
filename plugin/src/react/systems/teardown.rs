//! React root teardown when a [`ReactRoot`] component is removed or its entity despawned.

use bevy::prelude::*;

use crate::js_bevy::JsClientResource;
use crate::react::systems::types::*;

/// Observer: clean Bevy + JS state when a `ReactRoot` is removed (including despawn).
///
/// Runs while component data is still readable (`Remove` fires before the remove).
pub fn on_react_root_removed(
    remove: On<Remove, ReactRoot>,
    mut roots: Query<(
        &ReactRoot,
        Option<&mut ReactContext>,
        Option<&ReactScriptSource>,
    )>,
    mut root_map: ResMut<ReactRootMap>,
    mut focused: ResMut<FocusedNode>,
    mut commands: Commands,
    js_client: Option<Res<JsClientResource>>,
) {
    let entity = remove.entity;
    let Ok((root, context, source)) = roots.get_mut(entity) else {
        return;
    };

    let root_id = root.id.clone();
    root_map.roots.remove(&root_id);

    let node_entities: Vec<Entity> = context
        .as_ref()
        .map(|ctx| ctx.nodes.values().copied().collect())
        .unwrap_or_default();

    if focused
        .entity
        .is_some_and(|e| e == entity || node_entities.contains(&e))
    {
        *focused = FocusedNode::default();
    }

    // Despawn mapped host nodes (including orphans detached from the hierarchy).
    // Skip the root entity itself — it is already being removed/despawned.
    for node_entity in node_entities {
        if node_entity == entity {
            continue;
        }
        if let Ok(mut entity_commands) = commands.get_entity(node_entity) {
            entity_commands.try_despawn();
        }
    }

    if let Some(mut context) = context {
        context.nodes.clear();
        context.root = None;
    }

    // Notify JS so fiber / instance maps for this root are dropped. Host destroy
    // RPCs may no-op once the root is gone from ReactRootMap — that is intentional.
    let Some(js_client) = js_client else {
        log::debug!(
            "React root teardown: no JsClientResource; skipped JS unmount for {}",
            root_id
        );
        return;
    };
    let Some(source) = source else {
        log::debug!(
            "React root teardown: no ReactScriptSource; skipped JS unmount for {}",
            root_id
        );
        return;
    };

    js_client.execute(format!(
        r#"
            (async () => {{
                try {{
                    const mod = await import('{module}');
                    if (mod.default && typeof mod.default.unmount === 'function') {{
                        mod.default.unmount('{root_id}');
                    }} else if (typeof __react_unmount_root === 'function') {{
                        __react_unmount_root('{root_id}');
                    }}
                }} catch (err) {{
                    console.warn('[bevy-react] Failed to unmount root {root_id}:', err);
                }}
            }})()
        "#,
        module = &source.module_name,
        root_id = root_id,
    ));

    log::info!("Tore down React root {}", root_id);
}
