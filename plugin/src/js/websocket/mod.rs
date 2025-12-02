//! WebSocket Implementation for Boa JS Engine
//!
//! Provides a WebSocket shim that delegates to Rust's tokio-tungstenite.
//! This enables Vite HMR and other WebSocket-dependent features.
//!
//! Events are pushed from Rust to JS via JsEngineClient::execute(),
//! similar to how input events are dispatched.

mod manager;
mod extension;

pub use extension::WebSocketExtension;
