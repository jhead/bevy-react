use bevy::prelude::*;

use crate::react::{
    ReactContext, ReactDirtyFlag, ReactHmrRoot, ReactRoot, ReactScriptSource,
};

/// Load a React app from a Vite dev server with HMR support.
///
/// The generated bootstrap:
/// 1. Sets `NODE_ENV=development`
/// 2. Installs the React Refresh preamble
/// 3. Wraps `WebSocket` so Vite `update` / `full-reload` messages call
///    `__react_request_reload()` (re-sets [`ReactDirtyFlag`] on [`ReactHmrRoot`]s)
/// 4. Loads `@vite/client` and the app entry (cache-busted on each evaluation)
#[derive(Clone, Debug)]
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

    /// Spawn-ready bundle including the [`ReactHmrRoot`] marker.
    pub fn into_bundle(self, root_node: Node) -> impl Bundle {
        (
            ReactRoot::new(),
            root_node,
            ReactScriptSource::from(self),
            ReactContext::default(),
            ReactDirtyFlag,
            ReactHmrRoot,
        )
    }

    fn bootstrap_source(&self) -> String {
        let entry = self.entry_point.trim_start_matches('/');
        format!(
            r#"
                // Ensure React runs in development mode under Vite HMR.
                if (typeof process === 'undefined') {{
                    globalThis.process = {{ env: {{ NODE_ENV: 'development' }} }};
                }} else {{
                    process.env = process.env || {{}};
                    process.env.NODE_ENV = 'development';
                }}

                import RefreshRuntime from '{dev_server_url}/@react-refresh';
                RefreshRuntime.injectIntoGlobalHook(window);
                window.$RefreshReg$ = () => {{}};
                window.$RefreshSig$ = () => (type) => type;
                window.__vite_plugin_react_preamble_installed__ = true;

                // Intercept Vite HMR WebSocket messages → Bevy ReactDirtyFlag.
                (function installViteHmrBridge() {{
                    if (globalThis.__bevy_react_vite_hmr_bridged__) {{
                        return;
                    }}
                    globalThis.__bevy_react_vite_hmr_bridged__ = true;

                    const NativeWebSocket = globalThis.WebSocket;
                    if (typeof NativeWebSocket !== 'function') {{
                        console.warn('[bevy-react] WebSocket unavailable; Vite HMR reload bridge disabled');
                        return;
                    }}

                    let reloadTimer = null;
                    function scheduleReload(reason) {{
                        if (typeof __react_request_reload !== 'function') {{
                            console.warn('[bevy-react] __react_request_reload missing; HMR ignored (' + reason + ')');
                            return;
                        }}
                        if (reloadTimer !== null) {{
                            clearTimeout(reloadTimer);
                        }}
                        reloadTimer = setTimeout(function() {{
                            reloadTimer = null;
                            console.log('[bevy-react] Vite HMR → reload (' + reason + ')');
                            __react_request_reload();
                        }}, 50);
                    }}

                    globalThis.WebSocket = function(url, protocols) {{
                        const ws = protocols === undefined
                            ? new NativeWebSocket(url)
                            : new NativeWebSocket(url, protocols);

                        const urlStr = String(url);
                        const isVite =
                            urlStr.indexOf('@vite') !== -1 ||
                            urlStr.indexOf('vite-hmr') !== -1 ||
                            urlStr.indexOf(':5173') !== -1 ||
                            (Array.isArray(protocols) && protocols.indexOf('vite-hmr') !== -1) ||
                            protocols === 'vite-hmr';

                        if (isVite) {{
                            ws.addEventListener('message', function(event) {{
                                let msg;
                                try {{
                                    msg = JSON.parse(event.data);
                                }} catch (_) {{
                                    return;
                                }}
                                if (!msg || typeof msg.type !== 'string') {{
                                    return;
                                }}
                                if (msg.type === 'update' || msg.type === 'full-reload' || msg.type === 'prune') {{
                                    scheduleReload(msg.type);
                                }}
                            }});
                        }}

                        return ws;
                    }};
                    globalThis.WebSocket.prototype = NativeWebSocket.prototype;
                    Object.keys(NativeWebSocket).forEach(function(k) {{
                        try {{ globalThis.WebSocket[k] = NativeWebSocket[k]; }} catch (_) {{}}
                    }});
                    globalThis.WebSocket.CONNECTING = NativeWebSocket.CONNECTING;
                    globalThis.WebSocket.OPEN = NativeWebSocket.OPEN;
                    globalThis.WebSocket.CLOSING = NativeWebSocket.CLOSING;
                    globalThis.WebSocket.CLOSED = NativeWebSocket.CLOSED;
                }})();

                await import('{dev_server_url}/@vite/client');

                // Cache-bust the entry so each HMR re-evaluation fetches fresh modules.
                const app = await import('{dev_server_url}/{entry_point}?t=' + Date.now());
                export default app.default;
            "#,
            dev_server_url = self.dev_server_url.trim_end_matches('/'),
            entry_point = entry,
        )
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

impl From<ViteDevSource> for ReactScriptSource {
    fn from(value: ViteDevSource) -> Self {
        ReactScriptSource::from_string(value.module_name.clone(), value.bootstrap_source())
    }
}
