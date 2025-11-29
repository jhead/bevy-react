//! JavaScript RPC Protocol
//!
//! Defines the callback interface for JS-to-Rust communication.

use boa_engine::Context;
use std::sync::Arc;

/// Trait for registering native function callbacks into the JS context.
///
/// Implementors provide a way to register their native functions with the JS engine.
/// This allows different subsystems (React, game scripting, etc.) to expose their
/// own APIs without the JS engine knowing about them.
pub trait JsCallback: Send + Sync {
    /// Register native functions in the given JS context.
    fn register(&self, context: &mut Context);
}

/// Registry for managing multiple callback providers.
///
/// This allows multiple subsystems to register their callbacks with the engine.
#[derive(Default)]
pub struct JsCallbackRegistry {
    callbacks: Vec<Arc<dyn JsCallback>>,
}

impl JsCallbackRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a callback provider to the registry.
    pub fn register(&mut self, callback: Arc<dyn JsCallback>) {
        self.callbacks.push(callback);
    }

    /// Register all callbacks with the given JS context.
    pub fn register_all(&self, context: &mut Context) {
        for callback in &self.callbacks {
            callback.register(context);
        }
    }
}
