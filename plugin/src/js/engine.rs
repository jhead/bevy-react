//! JavaScript Engine (native)
//!
//! Manages the Boa JavaScript runtime on a dedicated worker thread.
//! Commands arrive via an `mpsc` channel; the thread blocks on `recv()` between frames.
//! The two-thread design allows Boa closures to capture `JsEngineClient` (an `mpsc::Sender`)
//! and synchronously request data from Bevy without needing globals or unsafe statics.

use boa_engine::{Context, JsError, Module, Source};
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::thread;

use crate::js::JsEngineClient;
use crate::js::builder::JsEngineExtension;
use crate::js::esm::FetchModuleLoader;

/// Commands that can be sent to the JS engine thread.
pub enum JsCommand {
    LoadEsmModule { name: String, source: String },
    Execute { source: String },
    RegisterExtension { extension: Arc<Box<dyn JsEngineExtension>> },
    FlushEventLoop,
    Shutdown,
}

/// JavaScript engine with a dedicated worker thread.
pub struct JsEngine {
    pub(crate) client: JsEngineClient,
    pub(crate) context_builder: Box<dyn FnOnce() -> Result<Context, JsError> + Send + Sync>,
    pub(crate) receiver: Receiver<JsCommand>,
}

impl JsEngine {
    /// Spawn the JS engine thread and return the client handle.
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

/// Main loop for the JS engine thread — blocks on `recv()` between commands.
fn run_js_loop(mut context: Context, receiver: Receiver<JsCommand>, client: JsEngineClient) {
    log::info!("JS runtime initialized");

    loop {
        match receiver.recv() {
            Ok(cmd) => {
                let is_shutdown = matches!(cmd, JsCommand::Shutdown);
                process_js_command(&mut context, cmd, &client);
                if is_shutdown {
                    break;
                }
            }
            Err(e) => {
                log::error!("JS engine channel error: {}", e);
                break;
            }
        }
    }

    log::info!("JS engine thread stopped");
}

fn process_js_command(context: &mut Context, cmd: JsCommand, client: &JsEngineClient) {
    match cmd {
        JsCommand::Execute { source } => {
            log::info!("Executing script ({} bytes)...", source.len());
            let source = Source::from_bytes(source.as_bytes());
            if let Err(e) = context.eval(source) {
                log::error!("Failed to execute script: {:?}", e);
            }
            flush_event_loop(context);
        }
        JsCommand::LoadEsmModule { name, source } => {
            log::info!("Loading ES module {} ({} bytes)...", name, source.len());
            let source = Source::from_bytes(source.as_bytes());
            let module = Module::parse(source, None, context).unwrap();
            let _promise = module.load_link_evaluate(context);
            if let Some(loader) = context.downcast_module_loader::<FetchModuleLoader>() {
                log::info!("Registering ESM module: {}", name);
                loader.insert(name, module);
            }
            flush_event_loop(context);
        }
        JsCommand::RegisterExtension { extension } => {
            if let Err(e) = extension.register(context, client.clone()) {
                log::error!("Failed to register extension: {:?}", e);
            }
            flush_event_loop(context);
        }
        JsCommand::FlushEventLoop => {
            flush_event_loop(context);
        }
        JsCommand::Shutdown => {
            log::info!("JS engine shutting down");
        }
    }
}

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
