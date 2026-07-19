//! JavaScript Engine (native)
//!
//! Manages the Boa JavaScript runtime on a dedicated worker thread.
//! Commands arrive via an `mpsc` channel; the thread blocks on `recv()` between frames.
//! The two-thread design allows Boa closures to capture `JsEngineClient` (an `mpsc::Sender`)
//! and synchronously request data from Bevy without needing globals or unsafe statics.
//!
//! Panics inside the JS loop rebuild the Boa context and re-register extensions when
//! reasonable (channel still open, not shutting down).

use boa_engine::{Context, JsError, Module, Source};
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::thread;

use crate::js::JsEngineClient;
use crate::js::builder::JsEngineExtension;
use crate::js::error_report::{JsErrorSource, format_js_error};
use crate::js::esm::FetchModuleLoader;

/// Commands that can be sent to the JS engine thread.
pub enum JsCommand {
    LoadEsmModule { name: String, source: String },
    ClearEsmModuleCache,
    Execute { source: String },
    RegisterExtension { extension: Arc<Box<dyn JsEngineExtension>> },
    FlushEventLoop,
    Shutdown,
}

enum LoopExit {
    Shutdown,
    ChannelClosed,
}

/// JavaScript engine with a dedicated worker thread.
pub struct JsEngine {
    pub(crate) client: JsEngineClient,
    /// Rebuildable context factory — invoked on start and after panic recovery.
    pub(crate) context_builder: Box<dyn Fn() -> Result<Context, JsError> + Send + Sync>,
    pub(crate) receiver: Receiver<JsCommand>,
}

impl JsEngine {
    /// Spawn the JS engine thread and return the client handle.
    ///
    /// The worker restarts the Boa context if the command loop panics, re-applying
    /// any extensions registered via [`JsCommand::RegisterExtension`].
    pub fn start(self) -> Result<JsEngineClient, JsError> {
        let client_handle = self.client.clone();
        let client = self.client.clone();
        let reporter = client.reporter.clone();

        thread::spawn(move || {
            let mut registered_extensions: Vec<Arc<Box<dyn JsEngineExtension>>> = Vec::new();
            // generation 0 is the initial start; bump before each rebuild after panic
            let mut started = false;

            loop {
                if started {
                    reporter.bump_generation();
                    log::warn!(
                        "Restarting JS engine context (generation {})",
                        reporter.generation()
                    );
                }
                started = true;

                let context = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    (self.context_builder)()
                })) {
                    Ok(Ok(ctx)) => ctx,
                    Ok(Err(e)) => {
                        reporter.report_message(
                            JsErrorSource::Panic,
                            format!("Failed to build JS context: {e:?}"),
                            None,
                        );
                        break;
                    }
                    Err(panic) => {
                        reporter.report_message(
                            JsErrorSource::Panic,
                            format!("JS context builder panicked: {panic:?}"),
                            None,
                        );
                        // Retry build on next iteration.
                        continue;
                    }
                };

                let mut context = context;
                for ext in &registered_extensions {
                    if let Err(e) = ext.register(&mut context, client.clone()) {
                        log::error!("Failed to re-register extension after restart: {e:?}");
                    }
                }

                let exit = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    run_js_loop(
                        &mut context,
                        &self.receiver,
                        &client,
                        &mut registered_extensions,
                    )
                }));

                match exit {
                    Ok(LoopExit::Shutdown) | Ok(LoopExit::ChannelClosed) => break,
                    Err(panic) => {
                        reporter.report_message(
                            JsErrorSource::Panic,
                            format!("JS engine panicked: {panic:?}"),
                            None,
                        );
                        // Rebuild context and continue serving the same channel.
                        continue;
                    }
                }
            }

            log::info!("JS engine thread stopped");
        });

        Ok(client_handle)
    }
}

/// Main loop for the JS engine thread — blocks on `recv()` between commands.
fn run_js_loop(
    context: &mut Context,
    receiver: &Receiver<JsCommand>,
    client: &JsEngineClient,
    registered_extensions: &mut Vec<Arc<Box<dyn JsEngineExtension>>>,
) -> LoopExit {
    log::info!("JS runtime initialized");

    loop {
        match receiver.recv() {
            Ok(cmd) => {
                let is_shutdown = matches!(cmd, JsCommand::Shutdown);
                process_js_command(context, cmd, client, registered_extensions);
                if is_shutdown {
                    return LoopExit::Shutdown;
                }
            }
            Err(e) => {
                log::error!("JS engine channel error: {e}");
                return LoopExit::ChannelClosed;
            }
        }
    }
}

fn process_js_command(
    context: &mut Context,
    cmd: JsCommand,
    client: &JsEngineClient,
    registered_extensions: &mut Vec<Arc<Box<dyn JsEngineExtension>>>,
) {
    match cmd {
        JsCommand::Execute { source } => {
            log::info!("Executing script ({} bytes)...", source.len());
            let source = Source::from_bytes(source.as_bytes());
            if let Err(e) = context.eval(source) {
                let (message, stack) = format_js_error(&e, context);
                client.reporter.report_message(
                    JsErrorSource::Script,
                    message,
                    stack,
                );
            }
            flush_event_loop(context, client);
        }
        JsCommand::LoadEsmModule { name, source } => {
            log::info!("Loading ES module {name} ({} bytes)...", source.len());
            let source = Source::from_bytes(source.as_bytes());
            match Module::parse(source, None, context) {
                Ok(module) => {
                    let _promise = module.load_link_evaluate(context);
                    if let Some(loader) = context.downcast_module_loader::<FetchModuleLoader>() {
                        log::info!("Registering ESM module: {name}");
                        loader.insert(name, module);
                    }
                }
                Err(e) => {
                    let (message, stack) = format_js_error(&e, context);
                    client.reporter.report_message(
                        JsErrorSource::ModuleLoad,
                        format!("Failed to parse ESM module {name}: {message}"),
                        stack,
                    );
                }
            }
            flush_event_loop(context, client);
        }
        JsCommand::ClearEsmModuleCache => {
            if let Some(loader) = context.downcast_module_loader::<FetchModuleLoader>() {
                loader.clear();
            }
            flush_event_loop(context, client);
        }
        JsCommand::RegisterExtension { extension } => {
            if let Err(e) = extension.register(context, client.clone()) {
                log::error!("Failed to register extension: {e:?}");
            } else {
                registered_extensions.push(extension);
            }
            flush_event_loop(context, client);
        }
        JsCommand::FlushEventLoop => {
            flush_event_loop(context, client);
        }
        JsCommand::Shutdown => {
            log::info!("JS engine shutting down");
        }
    }
}

fn flush_event_loop(context: &mut Context, client: &JsEngineClient) {
    if let Err(e) = context.run_jobs() {
        let (message, stack) = format_js_error(&e, context);
        client
            .reporter
            .report_message(JsErrorSource::Job, message, stack);
    }
}
