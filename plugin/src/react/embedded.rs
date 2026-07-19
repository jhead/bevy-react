//! Compile-time embedded JS bundles via `include_str!`.

use crate::react::ReactScriptSource;

/// A prebuilt ESM bundle embedded into the binary at compile time.
///
/// # Example
///
/// ```ignore
/// use bevy_react::{EmbeddedBundleSource, ReactBundle, ReactScriptSource};
///
/// let source = EmbeddedBundleSource::new(
///     "my-app",
///     include_str!("../assets/ui/app.js"),
/// );
/// commands.spawn(ReactBundle::new(Node::default(), source.into()));
/// ```
#[derive(Clone, Debug)]
pub struct EmbeddedBundleSource {
    pub module_name: String,
    pub source: &'static str,
}

impl EmbeddedBundleSource {
    pub fn new(module_name: impl Into<String>, source: &'static str) -> Self {
        Self {
            module_name: module_name.into(),
            source,
        }
    }

    pub fn with_module_name(mut self, module_name: impl Into<String>) -> Self {
        self.module_name = module_name.into();
        self
    }
}

impl From<EmbeddedBundleSource> for ReactScriptSource {
    fn from(value: EmbeddedBundleSource) -> Self {
        ReactScriptSource::from_string(value.module_name, value.source)
    }
}
