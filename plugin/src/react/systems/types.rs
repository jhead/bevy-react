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

    pub fn from_path(path: impl Into<String>) -> Result<Self, io::Error> {
        let abs_path = Path::new(&path.into()).canonicalize()?;
        let module_name = abs_path.to_string_lossy();

        match fs::read_to_string(&abs_path) {
            Ok(content) => Ok(Self::from_string(module_name, content)),
            Err(e) => {
                log::error!("Failed to load js bundle from {}: {}", abs_path.display(), e);
                return Err(e);
            }
        }
    }

    /// Create a script source for loading from a Vite dev server with HMR support.
    ///
    /// This generates a bootstrap script that:
    /// 1. Loads the Vite HMR client (`/@vite/client`)
    /// 2. Loads the specified entry point
    ///
    /// # Arguments
    /// * `entry_point` - The path to the entry point relative to Vite root (e.g., "/src/index.tsx")
    /// * `vite_url` - The Vite dev server URL (default: "http://localhost:5173")
    ///
    /// # Example
    /// ```ignore
    /// let source = ReactScriptSource::from_vite("/src/index.tsx", None);
    /// ```
    pub fn from_vite(entry_point: impl Into<String>, vite_url: Option<&str>) -> Self {
        let base_url = vite_url.unwrap_or("http://localhost:5173");
        let entry = entry_point.into();
        let module_name = format!("{}{}", base_url, entry);

        // Bootstrap script that loads Vite client for HMR, then the entry point
        let source = format!(
            r#"
// Vite HMR Bootstrap
(async function() {{
    console.log('[Vite] Loading HMR client...');
    
    // Load Vite HMR client first
    try {{
        await import('{base_url}/@vite/client');
        console.log('[Vite] HMR client loaded');
    }} catch (e) {{
        console.warn('[Vite] Could not load HMR client:', e);
    }}
    
    // Load the entry point
    console.log('[Vite] Loading entry point: {entry}');
    const mod = await import('{base_url}{entry}');
    
    if (!mod.default) {{
        throw new Error('Entry point does not have a default export');
    }}
    
    console.log('[Vite] Entry point loaded');
}})();
"#,
            base_url = base_url,
            entry = entry
        );

        Self {
            module_name,
            source_string: source,
        }
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
