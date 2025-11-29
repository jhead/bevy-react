use crate::react::ReactScriptSource;

pub struct ViteDevSource {
    pub module_name: String,
    pub dev_server_url: String,
    pub entry_point: String,
}

impl ViteDevSource {
    pub fn with_module_name(mut self, module_name: impl Into<String>) -> Self {
        self.module_name = module_name.into();
        self
    }

    pub fn with_dev_server_url(mut self, dev_server_url: impl Into<String>) -> Self {
        self.dev_server_url = dev_server_url.into();
        self
    }

    pub fn with_entry_point(mut self, entry_point: impl Into<String>) -> Self {
        self.entry_point = entry_point.into();
        self
    }
}

impl Default for ViteDevSource {
    fn default() -> Self {
        Self {
            module_name: "bevy-react-entrypoint".to_string(),
            dev_server_url: "http://localhost:5173".to_string(),
            entry_point: "src/index.tsx".to_string(),
        }
    }
}

impl Into<ReactScriptSource> for ViteDevSource {
    fn into(self) -> ReactScriptSource {
        ReactScriptSource::from_string(
            self.module_name,
            format!(
                r#"
                import RefreshRuntime from '{dev_server_url}/@react-refresh'
                RefreshRuntime.injectIntoGlobalHook(window)
                window.$RefreshReg$ = () => {{}}
                window.$RefreshSig$ = () => (type) => type
                window.__vite_plugin_react_preamble_installed__ = true
                
                await import('{dev_server_url}/@vite/client');

                const app = await import('{dev_server_url}/{entry_point}');
                export default app.default;
            "#,
                dev_server_url = self.dev_server_url,
                entry_point = self.entry_point,
            ),
        )
    }
}
