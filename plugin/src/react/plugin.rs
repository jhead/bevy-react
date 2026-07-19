//! React Plugin for Bevy
//!
//! Provides infrastructure for React UI rendering using the js_bevy plugin.

use bevy::prelude::*;

use crate::js_bevy::{JsClientResource, JsEngineExtensionComponent};
use crate::react::ReactClient;
use crate::react::asset_source::{
    ReactJsModule, ReactJsModuleLoader, reload_modified_react_assets, resolve_react_assets,
};
use crate::react::bridge::{ReactBridge, flush_react_bridge, process_react_bridge_calls};
use crate::react::components_registry::{
    apply_react_bundles, BundleRegistry, ReactEntityMap,
};
use crate::react::event_queue::ReactEventQueue;
use crate::react::hmr::{ReactReloadFlag, apply_react_hmr_reloads};
use crate::react::native_functions::ReactJsExtension;
use crate::react::systems::*;
use crate::react::widgets::add_widget_plugins;

/// Loads an asset path into [`ReactDefaultFont`] at startup.
///
/// ```ignore
/// App::new().add_plugins((ReactPlugin, ReactDefaultFontPlugin::new("fonts/FiraSans.ttf")));
/// ```
pub struct ReactDefaultFontPlugin {
    path: String,
}

impl ReactDefaultFontPlugin {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }
}

impl Plugin for ReactDefaultFontPlugin {
    fn build(&self, app: &mut App) {
        let path = self.path.clone();
        app.init_resource::<ReactDefaultFont>().add_systems(
            Startup,
            move |mut fonts: ResMut<ReactDefaultFont>, asset_server: Res<AssetServer>| {
                fonts.0 = Some(asset_server.load(path.clone()));
            },
        );
    }
}

pub struct ReactPlugin;

impl Plugin for ReactPlugin {
    fn build(&self, app: &mut App) {
        log::info!("Building React plugin...");

        add_widget_plugins(app);

        app.init_asset::<ReactJsModule>()
            .init_asset_loader::<ReactJsModuleLoader>()
            .init_resource::<ReactRootMap>()
            .init_resource::<FocusedNode>()
            .init_resource::<ReactEventQueue>()
            .init_resource::<ReactBridge>()
            .init_resource::<ReactReloadFlag>()
            .init_resource::<ReactDefaultFont>()
            .init_resource::<BundleRegistry>()
            .init_resource::<ReactEntityMap>()
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
                    ApplyDeferred,
                    apply_react_bundles,
                    process_react_bridge_calls,
                    handle_input_interactions,
                    handle_pointer_move,
                    handle_click_outside_blur,
                    handle_wheel_scroll,
                    apply_focus_requests,
                    apply_interaction_styles,
                    handle_keyboard_input,
                    flush_react_events,
                    flush_react_bridge,
                    inspect,
                )
                    .chain(),
            );

        #[cfg(feature = "devtools")]
        {
            app.add_plugins(crate::react::devtools::ReactDevToolsPlugin);
        }

        #[cfg(feature = "egui")]
        {
            app.add_plugins(crate::react::devtools::ReactNodeInspectorPlugin);
        }

        log::info!("React plugin configured");
    }
}

/// System to register the React extension with the JS engine
fn register_react_extension(
    mut commands: Commands,
    event_queue: Res<ReactEventQueue>,
    bridge: Res<ReactBridge>,
    reload_flag: Res<ReactReloadFlag>,
    entity_map: Res<ReactEntityMap>,
) {
    let (client, receiver) = ReactClient::new();

    let react_ext = ReactJsExtension::new(
        client,
        event_queue.clone(),
        bridge.clone(),
        reload_flag.clone(),
        entity_map.clone(),
    );
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
    }
}
