//! JavaScript Bevy Plugin
//!
//! Provides Bevy integration for the JavaScript engine.
//! This plugin manages the JS engine lifecycle and exposes it as Bevy resources.

mod plugin;

pub use plugin::{JsEngineExtensionComponent, JsClientResource, JsPlugin};

// Re-export core JS types for convenience
pub use crate::js::{JsCommand, JsEngine, JsEngineClient};
