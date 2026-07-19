//! JavaScript Bevy Plugin Implementation

use bevy::prelude::*;
use std::{ops::Deref, sync::Arc};

use crate::js::{
    JsEngineBuilder, JsEngineClient, JsEngineExtension, JsErrorRecord, JsErrorSource,
};
#[cfg(feature = "websocket")]
use crate::js::WebSocketExtension;

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

/// Visible Bevy-side error state for JS script/module/console/uncaught failures
/// and native engine panic restarts.
///
/// Updated each frame from the shared [`crate::js::JsErrorReporter`]. Apps can
/// read `last_error` to show an overlay, and watch `engine_generation` to detect
/// native JS-thread restarts (re-dirty React roots as needed).
#[derive(Resource, Debug, Clone, Default)]
pub struct JsRuntimeError {
    pub last_error: Option<JsErrorRecord>,
    pub engine_generation: u64,
}

impl JsRuntimeError {
    pub fn clear(&mut self) {
        self.last_error = None;
    }

    pub fn source(&self) -> Option<JsErrorSource> {
        self.last_error.as_ref().map(|e| e.source)
    }
}

/// Bevy plugin for JavaScript engine integration.
///
/// This plugin:
/// - Starts the JS engine on a dedicated thread
/// - Exposes `JsClientResource` as a Bevy resource for script execution
/// - Ticks the JS event loop each frame
/// - Syncs [`JsRuntimeError`] from the JS error reporter
/// - Shuts down the engine on [`AppExit`]
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

        let builder = JsEngineBuilder::new();

        #[cfg(feature = "websocket")]
        let builder = builder.with_extension(WebSocketExtension {});

        let engine = builder.build().unwrap();

        let client = engine.start().unwrap();

        app.insert_resource(JsClientResource(client))
            .init_resource::<JsRuntimeError>()
            .add_systems(Update, (tick_js_engine, sync_js_runtime_error))
            .add_systems(Update, on_extension_added)
            .add_systems(Last, shutdown_js_engine_on_exit);
    }
}

/// Tick the JS event loop each frame.
fn tick_js_engine(client: Res<JsClientResource>) {
    client.flush_event_loop();
}

/// Pull the latest JS error (and engine generation) into a Bevy resource.
fn sync_js_runtime_error(
    client: Res<JsClientResource>,
    mut runtime_error: ResMut<JsRuntimeError>,
) {
    runtime_error.engine_generation = client.error_reporter().generation();
    if let Some(record) = client.error_reporter().take() {
        runtime_error.last_error = Some(record);
    }
}

fn shutdown_js_engine_on_exit(
    mut exits: MessageReader<AppExit>,
    client: Res<JsClientResource>,
) {
    if exits.is_empty() {
        return;
    }
    // Consume so we only shut down once even if multiple exit messages arrive.
    for _ in exits.read() {}
    log::info!("AppExit received — shutting down JS engine");
    client.shutdown();
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
