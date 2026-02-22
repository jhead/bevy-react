use std::rc::Rc;

#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc::{self, Receiver};
#[cfg(target_arch = "wasm32")]
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use boa_engine::{Context, JsError};

/// WASM-compatible clock for Boa — uses js_sys::Date::now() instead of
/// std::time::SystemTime::now(), which panics on wasm32-unknown-unknown.
#[cfg(target_arch = "wasm32")]
struct WasmClock;

#[cfg(target_arch = "wasm32")]
impl boa_engine::context::time::Clock for WasmClock {
    fn now(&self) -> boa_engine::context::time::JsInstant {
        let millis = js_sys::Date::now() as u64;
        boa_engine::context::time::JsInstant::new(millis / 1000, ((millis % 1000) * 1_000_000) as u32)
    }
}
use boa_runtime::extensions::{ConsoleExtension, MicrotaskExtension, TimeoutExtension};

#[cfg(not(target_arch = "wasm32"))]
use crate::js::JsCommand;
use crate::js::{JsEngine, JsEngineClient, esm::FetchModuleLoader};

pub struct JsEngineBuilder {
    extensions: Vec<Box<dyn JsEngineExtension>>,
    client: JsEngineClient,
    #[cfg(not(target_arch = "wasm32"))]
    receiver: Receiver<JsCommand>,
}

impl JsEngineBuilder {
    pub fn new() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let (sender, receiver) = mpsc::channel::<JsCommand>();

        JsEngineBuilder {
            extensions: vec![],
            client: JsEngineClient {
                #[cfg(not(target_arch = "wasm32"))]
                sender,
                // WASM: context slot starts empty; filled by JsEngine::start().
                #[cfg(target_arch = "wasm32")]
                context: Arc::new(Mutex::new(None)),
                // WASM: command queue instead of an mpsc channel.
                #[cfg(target_arch = "wasm32")]
                queue: Arc::new(Mutex::new(VecDeque::new())),
            },
            #[cfg(not(target_arch = "wasm32"))]
            receiver,
        }
    }

    pub fn with_extension(mut self, extension: impl JsEngineExtension) -> Self {
        self.extensions.push(Box::new(extension));
        self
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn build(self) -> Result<JsEngine, JsError> {
        let client = self.client.clone();
        let extensions = self.extensions;

        Ok(JsEngine {
            client: self.client,
            context_builder: Box::new(move || build_context(&extensions, client.clone())),
            receiver: self.receiver,
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub fn build(self) -> Result<JsEngine, JsError> {
        // Build and register extensions synchronously — the client's queue is available
        // for any extension that enqueues commands during registration; they'll be
        // processed on the first flush_event_loop call after start().
        let context = build_context(&self.extensions, self.client.clone())?;

        Ok(JsEngine {
            client: self.client,
            context,
        })
    }
}

pub trait JsEngineExtension: Send + Sync + 'static {
    fn register(&self, context: &mut Context, client: JsEngineClient) -> Result<(), JsError>;
}

fn build_context(
    extensions: &Vec<Box<dyn JsEngineExtension>>,
    client: JsEngineClient,
) -> Result<Context, JsError> {
    #[cfg(not(target_arch = "wasm32"))]
    let context_builder = Context::builder()
        .module_loader(Rc::new(FetchModuleLoader::new()));

    #[cfg(target_arch = "wasm32")]
    let context_builder = Context::builder()
        .module_loader(Rc::new(FetchModuleLoader::new()))
        .clock(Rc::new(WasmClock));

    let mut context = context_builder.build()?;

    boa_runtime::register(
        (
            ConsoleExtension::default(),
            TimeoutExtension {},
            MicrotaskExtension {},
        ),
        None,
        &mut context,
    )?;

    for extension in extensions {
        extension.register(&mut context, client.clone())?;
    }

    Ok(context)
}
