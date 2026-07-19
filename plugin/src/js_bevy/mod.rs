//! JavaScript Bevy Plugin
//!
//! Provides Bevy integration for the JavaScript engine.
//! This plugin manages the JS engine lifecycle and exposes it as Bevy resources.
//!
//! Includes a built-in in-game error overlay that reads [`JsRuntimeError`].

mod error_overlay;
mod plugin;

pub use plugin::{JsEngineExtensionComponent, JsClientResource, JsPlugin, JsRuntimeError};

// Re-export core JS types for convenience
pub use crate::js::{
    JsCommand, JsEngine, JsEngineClient, JsErrorRecord, JsErrorReporter, JsErrorSource,
};
