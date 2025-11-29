//! JavaScript Engine Module
//!
//! A pure Boa JS engine wrapper with RPC interface.
//! No Bevy dependencies - this can be used standalone.

mod engine;
mod protocol;
mod websocket;

pub use engine::{JsEngine, JsEngineClient, JsCommand};
pub use protocol::{JsCallback, JsCallbackRegistry};
pub use websocket::WebSocketJsCallback;

