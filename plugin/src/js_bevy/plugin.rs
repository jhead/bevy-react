//! JavaScript Bevy Plugin Implementation

use bevy::prelude::*;
use std::ops::Deref;
use std::sync::{Arc, Mutex};

use crate::js::{JsCallback, JsCallbackRegistry, JsEngine, JsEngineClient};

/// Configuration for the JavaScript plugin.
pub struct JsPluginConfig {
    /// Callbacks to register with the JS engine (wrapped for interior mutability).
    callbacks: Mutex<Option<JsCallbackRegistry>>,
}

impl Default for JsPluginConfig {
    fn default() -> Self {
        Self {
            callbacks: Mutex::new(Some(JsCallbackRegistry::new())),
        }
    }
}

impl JsPluginConfig {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a callback provider with the JS engine.
    pub fn with_callback(self, callback: Arc<dyn JsCallback>) -> Self {
        if let Ok(mut guard) = self.callbacks.lock() {
            if let Some(ref mut callbacks) = *guard {
                callbacks.register(callback);
            }
        }
        self
    }

    /// Take the callbacks out of the config.
    fn take_callbacks(&self) -> JsCallbackRegistry {
        self.callbacks
            .lock()
            .ok()
            .and_then(|mut guard| guard.take())
            .unwrap_or_default()
    }
}

/// Bevy Resource wrapper for JsEngineClient.
///
/// This allows the JsEngineClient to be used as a Bevy Resource while
/// keeping the js module free of Bevy dependencies.
#[derive(Resource, Clone)]
pub struct JsClientResource(JsEngineClient);

impl JsClientResource {
    /// Get the inner JsEngineClient.
    pub fn inner(&self) -> &JsEngineClient {
        &self.0
    }
}

impl Deref for JsClientResource {
    type Target = JsEngineClient;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Bevy Resource wrapper for JsEngine (keeps the thread alive).
#[derive(Resource)]
struct JsEngineResource(JsEngine);

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
///     .add_plugins(JsPlugin::new(JsPluginConfig::new()))
///     .run();
/// ```
pub struct JsPlugin {
    config: JsPluginConfig,
}

impl JsPlugin {
    pub fn new(config: JsPluginConfig) -> Self {
        Self { config }
    }
}

impl Plugin for JsPlugin {
    fn build(&self, app: &mut App) {
        // Take ownership of callbacks from config
        let callbacks = self.config.take_callbacks();

        log::info!("Starting JS engine...");
        let engine = JsEngine::start(callbacks);
        let client = engine.client();

        // Store the client as a resource for systems to use
        app.insert_resource(JsClientResource(client));
        // Store the engine (keeps the thread alive)
        app.insert_resource(JsEngineResource(engine));

        app.add_systems(Update, tick_js_engine);

        log::info!("JS engine configured");
    }
}

/// Tick the JS event loop each frame.
fn tick_js_engine(client: Option<Res<JsClientResource>>) {
    if let Some(client) = client {
        client.tick();
    }
}
