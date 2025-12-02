//! React Plugin for Bevy
//!
//! Provides infrastructure for React UI rendering using the js_bevy plugin.

use bevy::prelude::*;

use crate::js_bevy::{JsClientResource, JsEngineExtensionComponent};
use crate::react::ReactClient;
use crate::react::native_functions::ReactJsExtension;
use crate::react::systems::*;

pub struct ReactPlugin;

impl Plugin for ReactPlugin {
    fn build(&self, app: &mut App) {
        log::info!("Building React plugin...");

        app.init_resource::<ReactRootMap>()
            .init_resource::<FocusedNode>()
            .add_systems(Startup, register_react_extension)
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

/// System to register the React extension with the JS engine
fn register_react_extension(mut commands: Commands) {
    let (client, receiver) = ReactClient::new();
    
    let react_ext = ReactJsExtension::new(client);
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
            script.source_string[..100.min(script.source_string.len())].to_string()
        );
    }
}
