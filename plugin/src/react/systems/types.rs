use std::path::Path;
use std::{fs, io};

use bevy::asset::uuid::Uuid;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use crate::react::client::ReactClientReceiver;

/// Marker component for React-managed UI nodes
#[derive(Component)]
pub struct ReactNode {
    pub node_id: u64,
}

/// Marker component for React text nodes
#[derive(Component)]
pub struct ReactTextNode;

/// Marker component for the React UI root container
#[derive(Component)]
pub struct ReactRoot {
    pub id: String,
}

impl Default for ReactRoot {
    fn default() -> Self {
        Self::new()
    }
}

impl ReactRoot {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
        }
    }
}

#[derive(Bundle)]
pub struct ReactBundle {
    root: ReactRoot,
    root_node: Node,
    source: ReactScriptSource,
    context: ReactContext,
}

impl ReactBundle {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(root_node: Node, source: ReactScriptSource) -> impl Bundle {
        (
            Self {
                root: ReactRoot::new(),
                root_node,
                source,
                context: ReactContext::default(),
            },
            ReactDirtyFlag,
        )
    }
}

#[derive(Clone, Debug, Component)]
pub struct ReactScriptSource {
    pub module_name: String,
    pub source_string: String,
}

impl ReactScriptSource {
    pub fn from_string(module_name: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            module_name: module_name.into(),
            source_string: source.into(),
        }
    }

    /// Embed a prebuilt bundle with `include_str!` (or any `&'static str`).
    ///
    /// Prefer [`crate::EmbeddedBundleSource`] when you want a named builder type.
    pub fn from_embedded(module_name: impl Into<String>, source: &'static str) -> Self {
        Self::from_string(module_name, source)
    }

    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, io::Error> {
        let abs_path = path.as_ref().canonicalize()?;
        let module_name = abs_path.to_string_lossy().into_owned();

        match fs::read_to_string(&abs_path) {
            Ok(content) => Ok(Self::from_string(module_name, content)),
            Err(e) => {
                log::error!("Failed to load js bundle from {}: {}", abs_path.display(), e);
                Err(e)
            }
        }
    }

    /// Pick Vite (debug) vs production source based on `debug_assertions`.
    ///
    /// Both arguments are evaluated. For fallible release loading (e.g. [`Self::from_path`]),
    /// use [`Self::auto_with`] so the release path is not constructed in debug builds.
    ///
    /// # Example
    /// ```ignore
    /// ReactScriptSource::auto(
    ///     ViteDevSource::default().with_entry_point("src/main.tsx"),
    ///     EmbeddedBundleSource::new("app", include_str!("../assets/ui/app.js")),
    /// )
    /// ```
    pub fn auto(dev: impl Into<Self>, release: impl Into<Self>) -> Self {
        if cfg!(debug_assertions) {
            let _ = release;
            dev.into()
        } else {
            let _ = dev;
            release.into()
        }
    }

    /// Like [`Self::auto`], but lazily constructs only the selected source.
    pub fn auto_with(dev: impl FnOnce() -> Self, release: impl FnOnce() -> Self) -> Self {
        if cfg!(debug_assertions) {
            dev()
        } else {
            release()
        }
    }

    /// Create a script source for loading from a Vite dev server with HMR support.
    ///
    /// Prefer [`crate::ViteDevSource`] (and [`ViteDevSource::into_bundle`]) for new code.
    pub fn from_vite(entry_point: impl Into<String>, vite_url: Option<&str>) -> Self {
        use crate::react::ViteDevSource;

        let mut source = ViteDevSource::default().with_entry_point(entry_point);
        if let Some(url) = vite_url {
            source = source.with_dev_server_url(url);
        }
        source.into()
    }
}

#[derive(Resource, Default, Clone, Debug)]
pub struct ReactRootMap {
    pub roots: HashMap<String, Entity>,
}

/// Resource that maps React node IDs to Bevy Entities
#[derive(Component, Default)]
pub struct ReactContext {
    /// Mapping from React node ID to Bevy Entity
    pub nodes: HashMap<u64, Entity>,
    /// The root container entity
    pub root: Option<Entity>,
}

#[derive(Component)]
pub struct ReactDirtyFlag;

#[derive(Resource)]
pub struct ReactMessageReceiver(pub ReactClientReceiver);

/// Marker component for focusable elements (like text inputs)
#[derive(Component)]
pub struct Focusable;

/// Resource tracking which React node currently has keyboard focus
#[derive(Resource, Default)]
pub struct FocusedNode {
    /// The React node ID that has focus (None if nothing focused)
    pub node_id: Option<u64>,
    /// The entity that has focus
    pub entity: Option<Entity>,
    /// The module name for the focused element (cached for keyboard events)
    pub module_name: Option<String>,
}
