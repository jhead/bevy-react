//! JavaScript Engine
//!
//! Manages the Boa JavaScript runtime with a dedicated worker thread
//! and proper event loop integration.

use boa_engine::{Context, JsError, Module, Source};
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::thread;

use crate::js::JsEngineClient;
use crate::js::builder::JsEngineExtension;
use crate::js::esm::FetchModuleLoader;

/// Commands that can be sent to the JS engine thread.
pub enum JsCommand {
    /// Load an ESM module with the given name and source.
    LoadEsmModule { name: String, source: String },
    /// Execute a JS script (non-module).
    Execute { source: String },
    /// Register an extension with the JS engine.
    RegisterExtension {
        extension: Arc<Box<dyn JsEngineExtension>>,
    },
    /// Flush the event loop (run pending jobs and timers).
    FlushEventLoop,
    /// Shutdown the JS engine.
    Shutdown,
}

/// JavaScript engine with dedicated worker thread.
pub struct JsEngine {
    pub(crate) client: JsEngineClient,
    pub(crate) context_builder: Box<dyn FnOnce() -> Result<Context, JsError> + Send + Sync>,
    pub(crate) receiver: Receiver<JsCommand>,
}

impl JsEngine {
    /// Start a new JS engine with the given callback registry.
    pub fn start(self) -> Result<JsEngineClient, JsError> {
        let client = self.client.clone();

        thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let context = (self.context_builder)().unwrap();
                run_js_loop(context, self.receiver, self.client.clone());
            }));

            if let Err(e) = result {
                log::error!("JS engine panicked: {:?}", e);
            }
        });

        Ok(client)
    }
}

/// Main loop for the JS engine thread.
fn run_js_loop(mut context: Context, receiver: Receiver<JsCommand>, client: JsEngineClient) {
    log::info!("JS runtime initialized");

    // Process commands
    loop {
        match receiver.recv() {
            Ok(JsCommand::Execute { source }) => {
                log::info!("Executing script ({} bytes)...", source.len());

                let source = Source::from_bytes(source.as_bytes());

                if let Err(e) = context.eval(source) {
                    log::error!("Failed to execute script: {:?}", e);
                }

                flush_event_loop(&mut context);
            }
            Ok(JsCommand::LoadEsmModule { name, source }) => {
                log::info!("Loading ES module {} ({} bytes)...", name, source.len());

                let source = Source::from_bytes(source.as_bytes());
                let module = Module::parse(source, None, &mut context).unwrap();
                let _promise = module.load_link_evaluate(&mut context);

                if let Some(loader) = context.downcast_module_loader::<FetchModuleLoader>() {
                    log::info!("Registering ESM module: {}", name);
                    loader.insert(name, module);
                }

                flush_event_loop(&mut context);
            }
            Ok(JsCommand::RegisterExtension { extension }) => {
                if let Err(e) = extension.register(&mut context, client.clone()) {
                    log::error!("Failed to register extension: {:?}", e);
                }
                flush_event_loop(&mut context);
            }
            Ok(JsCommand::FlushEventLoop) => {
                flush_event_loop(&mut context);
            }
            Ok(JsCommand::Shutdown) => {
                log::info!("JS engine shutting down");
                break;
            }
            Err(e) => {
                log::error!("JS engine channel error: {}", e);
                break;
            }
        }
    }

    log::info!("JS engine thread stopped");
}

/// Flush the event loop: process pending events, run microtasks (Jobs), and pending macrotasks (timers).
fn flush_event_loop(context: &mut Context) {
    if let Err(e) = context.run_jobs() {
        if let Some(e) = e.as_opaque() {
            let msg = e.to_json(context).unwrap_or_default();
            log::error!("Error running Boa jobs: {:?}", msg);
        } else {
            log::error!("Error running Boa jobs: {:?}", e);
        }
    }
}
