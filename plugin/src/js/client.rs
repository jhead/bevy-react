#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc;
use std::sync::Arc;
#[cfg(target_arch = "wasm32")]
use std::sync::Mutex;
#[cfg(target_arch = "wasm32")]
use std::collections::VecDeque;

use crate::js::error_report::JsErrorReporter;
use crate::js::{JsCommand, JsEngineExtension};

/// WASM: wraps `boa_engine::Context` to satisfy `Send + Sync` bounds required by `Arc<Mutex<T>>`.
///
/// Sound because WASM targets are single-threaded — the Mutex never actually contends.
#[cfg(target_arch = "wasm32")]
pub(crate) struct WasmContext(pub(crate) boa_engine::Context);

#[cfg(target_arch = "wasm32")]
unsafe impl Send for WasmContext {}
#[cfg(target_arch = "wasm32")]
unsafe impl Sync for WasmContext {}

/// Client handle for communicating with the JS engine.
///
/// On native, wraps an `mpsc::Sender` that dispatches commands to the dedicated JS thread.
/// On WASM, wraps a shared command queue drained synchronously on each `flush_event_loop` call,
/// driven by Bevy's frame update — no dedicated thread, no polling loop.
#[derive(Clone)]
pub struct JsEngineClient {
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) sender: mpsc::Sender<JsCommand>,

    /// The Boa context, set once by `JsEngine::start()`.
    #[cfg(target_arch = "wasm32")]
    pub(crate) context: Arc<Mutex<Option<WasmContext>>>,

    /// Commands enqueued by Bevy systems (or JS callbacks calling back into Rust).
    /// Drained synchronously on each `flush_event_loop` call.
    /// Kept separate from `context` so that JS-triggered `execute()` calls during
    /// flush don't deadlock — they lock this queue, not the context.
    #[cfg(target_arch = "wasm32")]
    pub(crate) queue: Arc<Mutex<VecDeque<JsCommand>>>,

    /// Shared sink for console / uncaught / script errors (polled into Bevy).
    pub(crate) reporter: JsErrorReporter,
}

impl JsEngineClient {
    /// Access the shared error reporter (console, rejections, script/module failures).
    pub fn error_reporter(&self) -> &JsErrorReporter {
        &self.reporter
    }

    /// Flush the JS event loop — called every Bevy frame by `tick_js_engine`.
    ///
    /// On native, signals the JS thread to run Boa's job queue.
    /// On WASM, drains the command queue and runs a budgeted job pump (promises,
    /// due timers, host futures) without blocking.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn flush_event_loop(&self) {
        if let Err(e) = self.sender.send(JsCommand::FlushEventLoop) {
            log::warn!("Failed to send flush event loop command: {e}");
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn flush_event_loop(&self) {
        // Drain the queue first, releasing the lock before we touch the context.
        // This allows JS callbacks that fire during command processing to enqueue
        // new commands (via execute/load_esm_module) without deadlocking.
        let commands: Vec<JsCommand> = {
            let mut q = self.queue.lock().unwrap();
            q.drain(..).collect()
        };

        if let Ok(mut ctx_opt) = self.context.lock() {
            if let Some(ctx) = ctx_opt.as_mut() {
                for cmd in commands {
                    wasm::process_command(&mut ctx.0, cmd, self);
                }
                wasm::flush_jobs(&mut ctx.0, self);
            }
        }
    }

    /// Load an ES module by name and source.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_esm_module(&self, name: impl Into<String>, source: impl Into<String>) {
        if let Err(e) = self.sender.send(JsCommand::LoadEsmModule {
            name: name.into(),
            source: source.into(),
        }) {
            log::error!("Failed to send load ESM module command: {e}");
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn load_esm_module(&self, name: impl Into<String>, source: impl Into<String>) {
        self.queue.lock().unwrap().push_back(JsCommand::LoadEsmModule {
            name: name.into(),
            source: source.into(),
        });
    }

    /// Drop all cached ESM modules so the next import re-fetches / re-parses.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn clear_esm_module_cache(&self) {
        if let Err(e) = self.sender.send(JsCommand::ClearEsmModuleCache) {
            log::error!("Failed to send clear ESM module cache command: {e}");
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn clear_esm_module_cache(&self) {
        self.queue
            .lock()
            .unwrap()
            .push_back(JsCommand::ClearEsmModuleCache);
    }

    /// Execute a JS script.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn execute(&self, source: impl Into<String>) {
        if let Err(e) = self.sender.send(JsCommand::Execute {
            source: source.into(),
        }) {
            log::error!("Failed to send execute command: {e}");
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn execute(&self, source: impl Into<String>) {
        self.queue
            .lock()
            .unwrap()
            .push_back(JsCommand::Execute { source: source.into() });
    }

    /// Register an extension with the JS engine.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn register_extension(&self, extension: Arc<Box<dyn JsEngineExtension>>) {
        if let Err(e) = self.sender.send(JsCommand::RegisterExtension { extension }) {
            log::error!("Failed to send register extension command: {e}");
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn register_extension(&self, extension: Arc<Box<dyn JsEngineExtension>>) {
        self.queue
            .lock()
            .unwrap()
            .push_back(JsCommand::RegisterExtension { extension });
    }

    /// Shutdown the JS engine.
    pub fn shutdown(&self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = self.sender.send(JsCommand::Shutdown);
        }
        #[cfg(target_arch = "wasm32")]
        {
            // Drop the context so in-flight job state cannot be polled again.
            if let Ok(mut ctx_opt) = self.context.lock() {
                *ctx_opt = None;
            }
            if let Ok(mut q) = self.queue.lock() {
                q.clear();
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use boa_engine::{Context, Module, Source};
    use crate::js::JsCommand;
    use crate::js::error_report::{JsErrorSource, format_js_error};
    use crate::js::esm::FetchModuleLoader;
    use super::JsEngineClient;

    /// Process a single command against the Boa context.
    ///
    /// Called with the context lock held but the queue lock released, so that
    /// any `client.execute()` calls triggered by JS code enqueue into the queue
    /// without deadlocking — they'll be picked up on the next frame.
    pub(super) fn process_command(context: &mut Context, cmd: JsCommand, client: &JsEngineClient) {
        match cmd {
            JsCommand::Execute { source } => {
                log::info!("Executing script ({} bytes)...", source.len());
                let src = Source::from_bytes(source.as_bytes());
                if let Err(e) = context.eval(src) {
                    let (message, stack) = format_js_error(&e, context);
                    client.reporter.report_message(
                        JsErrorSource::Script,
                        message,
                        stack,
                    );
                }
                flush_jobs(context, client);
            }
            JsCommand::LoadEsmModule { name, source } => {
                log::info!("Loading ES module {name} ({} bytes)...", source.len());
                let src = Source::from_bytes(source.as_bytes());
                match Module::parse(src, None, context) {
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
                flush_jobs(context, client);
            }
            JsCommand::ClearEsmModuleCache => {
                if let Some(loader) = context.downcast_module_loader::<FetchModuleLoader>() {
                    loader.clear();
                }
                flush_jobs(context, client);
            }
            JsCommand::RegisterExtension { extension } => {
                if let Err(e) = extension.register(context, client.clone()) {
                    log::error!("Failed to register extension: {e:?}");
                }
                flush_jobs(context, client);
            }
            JsCommand::FlushEventLoop | JsCommand::Shutdown => {}
        }
    }

    pub(super) fn flush_jobs(context: &mut Context, client: &JsEngineClient) {
        // Uses FrameJobExecutor (installed in builder on wasm32): budgeted pump of
        // ready promise/timeout/generic jobs plus a non-blocking poll of host
        // futures. Must not call SimpleJobExecutor's block_on path — that busy-spins
        // on wasm32 and the browser kills the script.
        if let Err(e) = context.run_jobs() {
            let (message, stack) = format_js_error(&e, context);
            client
                .reporter
                .report_message(JsErrorSource::Job, message, stack);
        }
    }
}
