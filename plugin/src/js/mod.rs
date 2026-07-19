//! JavaScript Engine Module
//!
//! A pure Boa JS engine wrapper with RPC interface.
//! No Bevy dependencies - this can be used standalone.

#[cfg(not(target_arch = "wasm32"))]
mod engine;
#[cfg(target_arch = "wasm32")]
mod engine_wasm;
mod client;
#[cfg(feature = "websocket")]
mod websocket;
mod esm;
mod builder;
mod error_report;
mod sourcemap_enrich;
mod console_log;
mod host_hooks;
#[cfg(feature = "fetch")]
mod fetch_api;

#[cfg(not(target_arch = "wasm32"))]
pub use engine::{JsEngine, JsCommand};
#[cfg(target_arch = "wasm32")]
pub use engine_wasm::{JsEngine, JsCommand};
pub use client::JsEngineClient;
pub use builder::{JsEngineBuilder, JsEngineExtension};
pub use error_report::{JsErrorRecord, JsErrorReporter, JsErrorSource};
#[cfg(feature = "websocket")]
pub use websocket::*;
