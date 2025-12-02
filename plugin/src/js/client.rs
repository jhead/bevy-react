use std::sync::{Arc, mpsc};

use crate::js::{JsCommand, JsEngineExtension};

/// Client handle for communicating with the JS engine thread.
#[derive(Clone)]
pub struct JsEngineClient {
    pub sender: mpsc::Sender<JsCommand>,
}

impl JsEngineClient {
    /// Send a tick command to flush the JS event loop.
    pub fn flush_event_loop(&self) {
        if let Err(e) = self.sender.send(JsCommand::FlushEventLoop) {
            log::warn!("Failed to send flush event loop command: {}", e);
        }
    }

    /// Load an ES module.
    pub fn load_esm_module(&self, name: impl Into<String>, source: impl Into<String>) {
        if let Err(e) = self.sender.send(JsCommand::LoadEsmModule {
            name: name.into(),
            source: source.into(),
        }) {
            log::error!("Failed to send load ESM module command: {}", e);
        }
    }

    /// Execute a script.
    pub fn execute(&self, source: impl Into<String>) {
        if let Err(e) = self.sender.send(JsCommand::Execute {
            source: source.into(),
        }) {
            log::error!("Failed to send execute command: {}", e);
        }
    }

    /// Register an extension with the JS engine.
    pub fn register_extension(&self, extension: Arc<Box<dyn JsEngineExtension>>) {
        if let Err(e) = self.sender.send(JsCommand::RegisterExtension { extension: extension.clone() }) {
            log::error!("Failed to send register extension command: {}", e);
        }
    }

    /// Shutdown the JS engine.
    pub fn shutdown(&self) {
        let _ = self.sender.send(JsCommand::Shutdown);
    }
}
