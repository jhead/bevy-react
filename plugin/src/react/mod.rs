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
//!     let js_source = ViteDevSource::default()
//!         .with_entry_point("src/main.tsx")
//!         .into();
//!
//!     commands.spawn(ReactBundle::new(
//!         Node {
//!             width: Val::Percent(50.0),
//!             height: Val::Percent(100.0),
//!             left: Val::Percent(50.0),
//!             top: Val::Percent(0.0),
//!             position_type: PositionType::Absolute,
//!             ..default()
//!         },
//!         js_source,
//!     ));
//! }
//! ```
pub mod plugin;

mod client;
mod native_functions;
mod style;
mod systems;
mod vite;

pub use plugin::ReactPlugin;
pub use client::*;
pub use systems::*;
pub use vite::*;
