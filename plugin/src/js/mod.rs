//! JavaScript Engine Module
//!
//! A pure Boa JS engine wrapper with RPC interface.
//! No Bevy dependencies - this can be used standalone.

mod engine;
mod client;
mod websocket;
mod esm;
mod builder;

pub use engine::{JsEngine, JsCommand};
pub use client::JsEngineClient;
pub use builder::{JsEngineBuilder, JsEngineExtension};
pub use websocket::*;
