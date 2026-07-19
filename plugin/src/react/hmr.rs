//! Vite HMR → Bevy re-render bridge.
//!
//! JS (`ViteDevSource` bootstrap) calls `__react_request_reload()` when the Vite
//! WebSocket delivers `update` / `full-reload` messages. A Bevy system then
//! re-inserts [`ReactDirtyFlag`] on roots marked [`ReactHmrRoot`].

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use bevy::prelude::*;
use boa_gc::{Finalize, Trace, empty_trace};

use crate::js_bevy::JsClientResource;
use crate::react::{ReactDirtyFlag, ReactScriptSource};

/// Marker for React roots that should re-execute when Vite HMR fires.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct ReactHmrRoot;

/// Shared flag set from the JS thread when Vite signals an update.
#[derive(Clone, Default, Resource, Finalize)]
pub struct ReactReloadFlag {
    pending: Arc<AtomicBool>,
    generation: Arc<AtomicU64>,
}

unsafe impl Trace for ReactReloadFlag {
    empty_trace!();
}

impl ReactReloadFlag {
    pub fn new() -> Self {
        Self::default()
    }

    /// Called from JS (`__react_request_reload`).
    pub fn request(&self) {
        self.pending.store(true, Ordering::SeqCst);
        self.generation.fetch_add(1, Ordering::SeqCst);
    }

    pub fn take(&self) -> bool {
        self.pending.swap(false, Ordering::SeqCst)
    }

    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }
}

/// Apply pending HMR reload requests: bust module identity, clear the ESM HTTP
/// cache, and re-set [`ReactDirtyFlag`] so [`execute_react_scripts`] runs again.
pub(crate) fn apply_react_hmr_reloads(
    mut commands: Commands,
    reload: Res<ReactReloadFlag>,
    js_client: Option<Res<JsClientResource>>,
    mut roots: Query<(Entity, &mut ReactScriptSource), With<ReactHmrRoot>>,
) {
    if !reload.take() {
        return;
    }

    let generation = reload.generation();
    log::info!("Applying Vite HMR reload (generation {generation})");

    if let Some(js_client) = js_client {
        js_client.clear_esm_module_cache();
    }

    for (entity, mut script) in &mut roots {
        let base = script
            .module_name
            .split("__hmr_")
            .next()
            .unwrap_or(script.module_name.as_str())
            .to_string();
        script.module_name = format!("{base}__hmr_{generation}");
        commands.entity(entity).insert(ReactDirtyFlag);
    }
}
