//! React Plugin for Bevy
//!
//! Provides infrastructure for React UI rendering using the js_bevy plugin.

use bevy::prelude::*;

use crate::js_bevy::{JsClientResource, JsPlugin, JsPluginConfig};
use crate::react::ReactClient;
use crate::react::native_functions::ReactJsCallback;
use crate::react::systems::*;

pub struct ReactPlugin;

impl Plugin for ReactPlugin {
    fn build(&self, app: &mut App) {
        log::info!("Building React plugin...");

        // Create the React client and receiver for RPC communication
        let (react_client, receiver) = ReactClient::new();

        // Create the React JS callback that registers native functions
        let react_callback = ReactJsCallback::new(react_client);

        // Configure and add the JS plugin with React and Gen3D callbacks
        let js_config = JsPluginConfig::new()
            .with_callback(react_callback);

        app.add_plugins(JsPlugin::new(js_config));

        // Store the React message receiver
        app.insert_resource(ReactMessageReceiver(receiver));

        app.init_resource::<ReactRootMap>()
            .init_resource::<FocusedNode>()
            .add_systems(Update, execute_react_scripts)
            .add_systems(
                Update,
                (
                    process_react_messages,
                    handle_input_interactions,
                    handle_keyboard_input,
                    inspect,
                ),
            );

        log::info!("React plugin configured");
    }
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
            script.source_string[..100.min(script.source_string.len())].to_string()
        );
    }
}
