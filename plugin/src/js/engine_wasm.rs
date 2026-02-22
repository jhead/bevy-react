//! JavaScript Engine for WASM
//!
//! WASM-compatible engine that runs synchronously on the main thread.
//! No dedicated thread, no polling loop — driven entirely by Bevy's frame
//! update calling `client.flush_event_loop()` each tick via `tick_js_engine`.
//!
//! The design collapses the native two-thread channel model into single-threaded
//! shared state: the command queue (`Arc<Mutex<VecDeque<JsCommand>>>`) plays the
//! role the channel played on native, and the Bevy frame tick drives execution
//! instead of a blocking `receiver.recv()` loop.

use boa_engine::{Context, JsError};
use std::sync::Arc;

use crate::js::JsEngineClient;
use crate::js::builder::JsEngineExtension;
use crate::js::client::WasmContext;

/// Commands that can be sent to the JS engine.
pub enum JsCommand {
    LoadEsmModule { name: String, source: String },
    Execute { source: String },
    RegisterExtension { extension: Arc<Box<dyn JsEngineExtension>> },
    FlushEventLoop,
    Shutdown,
}

/// JavaScript engine (WASM, main-thread execution).
pub struct JsEngine {
    pub(crate) client: JsEngineClient,
    pub(crate) context: Context,
}

impl JsEngine {
    /// Initialize the engine and return the client handle.
    ///
    /// Transfers ownership of the Boa `Context` into the shared slot inside the
    /// client so that subsequent `flush_event_loop` calls can drive execution.
    pub fn start(self) -> Result<JsEngineClient, JsError> {
        log::info!("JS runtime initialized (WASM mode)");

        let client = self.client.clone();
        if let Ok(mut ctx_opt) = client.context.lock() {
            *ctx_opt = Some(WasmContext(self.context));
        }

        Ok(client)
    }
}
