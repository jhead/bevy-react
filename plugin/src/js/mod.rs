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

#[cfg(not(target_arch = "wasm32"))]
pub use engine::{JsEngine, JsCommand};
#[cfg(target_arch = "wasm32")]
pub use engine_wasm::{JsEngine, JsCommand};
pub use client::JsEngineClient;
pub use builder::{JsEngineBuilder, JsEngineExtension};
#[cfg(feature = "websocket")]
pub use websocket::*;
