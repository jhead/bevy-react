//! React Plugin for Bevy
//!
//! Provides infrastructure for React UI rendering using the js_bevy plugin.

use bevy::prelude::*;

use crate::js_bevy::{JsClientResource, JsEngineExtensionComponent};
use crate::react::ReactClient;
use crate::react::asset_source::{
    ReactJsModule, ReactJsModuleLoader, reload_modified_react_assets, resolve_react_assets,
};
use crate::react::event_queue::ReactEventQueue;
use crate::react::hmr::{ReactReloadFlag, apply_react_hmr_reloads};
use crate::react::native_functions::ReactJsExtension;
use crate::react::systems::*;

pub struct ReactPlugin;

impl Plugin for ReactPlugin {
    fn build(&self, app: &mut App) {
        log::info!("Building React plugin...");

        app.init_asset::<ReactJsModule>()
            .init_asset_loader::<ReactJsModuleLoader>()
            .init_resource::<ReactRootMap>()
            .init_resource::<FocusedNode>()
            .init_resource::<ReactEventQueue>()
            .init_resource::<ReactReloadFlag>()
            .add_message::<RequestReactFocus>()
            .add_message::<RequestReactBlur>()
            .add_observer(on_react_root_removed)
            .add_systems(Startup, register_react_extension)
            .add_systems(
                Update,
                (
                    resolve_react_assets,
                    reload_modified_react_assets,
                    apply_react_hmr_reloads,
                    execute_react_scripts,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                (
                    process_react_messages,
                    handle_input_interactions,
                    handle_pointer_move,
                    handle_click_outside_blur,
                    handle_wheel_scroll,
                    apply_focus_requests,
                    handle_keyboard_input,
                    flush_react_events,
                    inspect,
                ),
            );

        log::info!("React plugin configured");
    }
}

/// System to register the React extension with the JS engine
fn register_react_extension(
    mut commands: Commands,
    event_queue: Res<ReactEventQueue>,
    reload_flag: Res<ReactReloadFlag>,
) {
    let (client, receiver) = ReactClient::new();

    let react_ext = ReactJsExtension::new(client, event_queue.clone(), reload_flag.clone());
    commands.spawn(JsEngineExtensionComponent::new(react_ext));
    commands.insert_resource(ReactMessageReceiver(receiver));
}

fn execute_react_scripts(
    mut commands: Commands,
    mut scripts: Query<
        (Entity, &ReactRoot, &ReactScriptSource, Mut<ReactContext>),
        With<ReactDirtyFlag>,
    >,
    js_client: Option<Res<JsClientResource>>,
    mut root_map: ResMut<ReactRootMap>,
) {
    let Some(js_client) = js_client else {
        return;
    };

    for (entity, root, script, mut context) in scripts.iter_mut() {
        js_client.load_esm_module(&script.module_name, &script.source_string);
        js_client.execute(format!(
            r#"
                (async () => {{
                    try {{
                        const mod = await import('{module}');
                        
                        if (!mod.default) {{
                            console.warn('Module does not have a default export', mod);
                            throw new Error('Module does not have a default export');
                        }}

                        mod.default.render('{root_id}');
                    }} catch (err) {{
                        console.error("Failed to load Bevy React app:", err);
                        if (err.stack) console.error(err.stack);
                    }}
                }})()
            "#,
            module = &script.module_name,
            root_id = root.id,
        ));

        context.root = Some(entity);
        root_map.roots.insert(root.id.clone(), entity);
        commands.entity(entity).remove::<ReactDirtyFlag>();

        log::info!(
            "Executed script: {}",
            &script.source_string[..100.min(script.source_string.len())]
        );
    }
}
