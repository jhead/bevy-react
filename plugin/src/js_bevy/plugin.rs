//! JavaScript Bevy Plugin Implementation

use bevy::prelude::*;
use std::{ops::Deref, sync::Arc};

use crate::js::{ JsEngineBuilder, JsEngineClient, JsEngineExtension, WebSocketExtension};

/// Bevy Resource wrapper for JsEngineClient.
///
/// This allows the JsEngineClient to be used as a Bevy Resource while
/// keeping the js module free of Bevy dependencies.
#[derive(Resource, Clone)]
pub struct JsClientResource(JsEngineClient);

impl Deref for JsClientResource {
    type Target = JsEngineClient;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Bevy plugin for JavaScript engine integration.
///
/// This plugin:
/// - Starts the JS engine on a dedicated thread
/// - Exposes `JsClientResource` as a Bevy resource for script execution
/// - Ticks the JS event loop each frame
///
/// ## Usage
///
/// ```ignore
/// App::new()
///     .add_plugins(JsPlugin)
///     .run();
/// ```
pub struct JsPlugin;

impl Plugin for JsPlugin {
    fn build(&self, app: &mut App) {
        log::info!("Starting JS engine...");

        let engine = JsEngineBuilder::new()
            .with_extension(WebSocketExtension {})
            .build()
            .unwrap();

        let client = engine.start().unwrap();

        app.insert_resource(JsClientResource(client))
            .add_systems(Update, tick_js_engine)
            .add_systems(Update, on_extension_added);
    }
}

/// Tick the JS event loop each frame.
fn tick_js_engine(client: Res<JsClientResource>) {
    client.flush_event_loop();
}

#[derive(Component)]
pub struct JsEngineExtensionComponent(pub Arc<Box<dyn JsEngineExtension>>);

impl JsEngineExtensionComponent {
    pub fn new<T: JsEngineExtension>(extension: T) -> Self {
        JsEngineExtensionComponent(Arc::new(Box::new(extension)))
    }
}

#[derive(Component)]
struct JsEngineExtensionRegistered;

fn on_extension_added(
    mut commands: Commands,
    client: Res<JsClientResource>,
    extensions: Query<(&JsEngineExtensionComponent, Entity), Without<JsEngineExtensionRegistered>>,
) {
    for (extension, entity) in extensions.iter() {
        client.0.register_extension(extension.0.clone());
        commands.entity(entity).insert(JsEngineExtensionRegistered);
    }
}
