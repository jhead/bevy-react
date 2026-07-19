//! # React UI Plugin for Bevy
//!
//! This module enables building Bevy UI using React, with bidirectional interoperability between Rust and JavaScript/TypeScript.
//!
//! ## Example
//!
//! ```rust
//! use bevy::prelude::*;
//! use bevy_react::{ReactPlugin, ReactBundle, ViteDevSource};
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins(ReactPlugin)
//!         .add_systems(Startup, setup)
//!         .run();
//! }
//!
//! fn setup(mut commands: Commands) {
//!     commands.spawn(Camera2d);
//!
//!     // Load the React app via Vite dev server with hot reloading
//!     commands.spawn(
//!         ViteDevSource::default()
//!             .with_entry_point("src/main.tsx")
//!             .into_bundle(Node {
//!                 width: Val::Percent(50.0),
//!                 height: Val::Percent(100.0),
//!                 left: Val::Percent(50.0),
//!                 top: Val::Percent(0.0),
//!                 position_type: PositionType::Absolute,
//!                 ..default()
//!             }),
//!     );
//! }
//! ```
pub mod plugin;

mod asset_source;
mod bridge;
mod client;
pub mod components_registry;
#[cfg(any(feature = "devtools", feature = "egui"))]
pub mod devtools;
mod embedded;
mod event_queue;
mod hmr;
mod native_functions;
mod style;
mod systems;
mod vite;
mod widgets;
mod shim;

#[cfg(test)]
mod message_tests;

pub use plugin::{ReactDefaultFontPlugin, ReactPlugin};
#[cfg(feature = "devtools")]
pub use devtools::{ReactDevToolsBridge, ReactDevToolsPlugin, DEVTOOLS_WS_ADDR, DEVTOOLS_WS_PORT};
#[cfg(feature = "egui")]
pub use devtools::ReactNodeInspectorPlugin;
pub use asset_source::{ReactAssetBundle, ReactAssetSource, ReactJsModule};
pub use bridge::{
    BridgeCall, BridgeCallResult, ReactBridge, flush_react_bridge,
    process_react_bridge_calls, sync_registered_query_stores,
    sync_registered_resource_stores,
};
pub use client::*;
pub use components_registry::{
    apply_react_bundles, BundleRegistry, ReactEntityMap,
};
pub use embedded::EmbeddedBundleSource;
pub use event_queue::{FLUSH_EVENTS_SCRIPT, ReactEvent, ReactEventQueue};
pub use native_functions::ReactJsExtension;
pub use hmr::{ReactHmrRoot, ReactReloadFlag};
pub use systems::*;
pub use vite::*;
