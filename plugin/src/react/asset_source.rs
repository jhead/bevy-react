//! Load prebuilt React JS bundles through Bevy's [`AssetServer`].
//!
//! Works with the default `assets/` folder, packed asset sources, and WASM.

use bevy::asset::{AssetLoader, LoadContext, io::Reader};
use bevy::prelude::*;

use crate::react::{ReactDirtyFlag, ReactScriptSource};

/// A UTF-8 JS/ESM module loaded as a Bevy asset (`.js` / `.mjs`).
#[derive(Asset, TypePath, Clone, Debug)]
pub struct ReactJsModule {
    pub source: String,
}

/// Asset loader for [`ReactJsModule`].
#[derive(Default)]
pub struct ReactJsModuleLoader;

impl AssetLoader for ReactJsModuleLoader {
    type Asset = ReactJsModule;
    type Settings = ();
    type Error = std::io::Error;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let source = String::from_utf8(bytes).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e.utf8_error())
        })?;
        Ok(ReactJsModule { source })
    }

    fn extensions(&self) -> &[&str] {
        &["js", "mjs"]
    }
}

/// Pending (or live) handle to a [`ReactJsModule`] asset for a React root.
///
/// When the asset becomes ready, [`resolve_react_assets`] inserts
/// [`ReactScriptSource`] and [`ReactDirtyFlag`]. Asset modifications re-apply
/// the source and mark the root dirty again.
#[derive(Component, Clone, Debug)]
pub struct ReactAssetSource {
    pub handle: Handle<ReactJsModule>,
    pub module_name: String,
}

impl ReactAssetSource {
    /// Begin loading `asset_path` from the [`AssetServer`].
    pub fn load(
        asset_server: &AssetServer,
        asset_path: impl Into<String>,
        module_name: impl Into<String>,
    ) -> Self {
        let asset_path = asset_path.into();
        Self {
            handle: asset_server.load::<ReactJsModule>(asset_path),
            module_name: module_name.into(),
        }
    }
}

/// Bundle helper: spawn a React root that loads its script via [`AssetServer`].
#[derive(Bundle)]
pub struct ReactAssetBundle {
    root: crate::react::ReactRoot,
    root_node: Node,
    asset_source: ReactAssetSource,
    context: crate::react::ReactContext,
}

impl ReactAssetBundle {
    pub fn new(
        root_node: Node,
        asset_server: &AssetServer,
        asset_path: impl Into<String>,
        module_name: impl Into<String>,
    ) -> Self {
        Self {
            root: crate::react::ReactRoot::new(),
            root_node,
            asset_source: ReactAssetSource::load(asset_server, asset_path, module_name),
            context: crate::react::ReactContext::default(),
        }
    }
}

/// Promote loaded [`ReactAssetSource`] handles into executable [`ReactScriptSource`].
pub(crate) fn resolve_react_assets(
    mut commands: Commands,
    assets: Res<Assets<ReactJsModule>>,
    pending: Query<(Entity, &ReactAssetSource), Without<ReactScriptSource>>,
) {
    for (entity, source) in &pending {
        let Some(module) = assets.get(&source.handle) else {
            continue;
        };

        commands.entity(entity).insert((
            ReactScriptSource::from_string(source.module_name.clone(), module.source.clone()),
            ReactDirtyFlag,
        ));

        log::info!(
            "Resolved React asset module '{}' ({} bytes)",
            source.module_name,
            module.source.len()
        );
    }
}

/// When a [`ReactJsModule`] asset is modified (e.g. hot-replaced on disk), refresh
/// attached roots and re-execute.
pub(crate) fn reload_modified_react_assets(
    mut commands: Commands,
    mut events: MessageReader<AssetEvent<ReactJsModule>>,
    assets: Res<Assets<ReactJsModule>>,
    mut roots: Query<(Entity, &ReactAssetSource, &mut ReactScriptSource)>,
) {
    for event in events.read() {
        let AssetEvent::Modified { id } = event else {
            continue;
        };

        for (entity, source, mut script) in &mut roots {
            if source.handle.id() != *id {
                continue;
            }
            let Some(module) = assets.get(&source.handle) else {
                continue;
            };

            script.source_string = module.source.clone();
            // Bust Boa module identity so the ESM cache re-evaluates.
            script.module_name = format!("{}__asset_{}", source.module_name, id);
            commands.entity(entity).insert(ReactDirtyFlag);

            log::info!(
                "Reloaded React asset module '{}' ({} bytes)",
                source.module_name,
                module.source.len()
            );
        }
    }
}
