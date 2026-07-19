//! React DevTools / inspector hooks (optional `devtools` / `egui` features).
//!
//! ## `devtools` feature
//!
//! Starts a JSON WebSocket server on `127.0.0.1:8098` that publishes:
//! - `ecs_map` — `ReactContext.nodes` (`node_id` ↔ Bevy `Entity`)
//! - `tree` — host instance tree from JS (`__bevyReactDevTools.dump()`)
//! - `fiber_tree` — reconciler fiber walk from JS
//! - RDT-shaped `{ event, payload }` envelopes (`operations`, `rendererAttached`, …)
//!
//! This is a **debug bridge** with React DevTools–inspired message *shapes*.
//! It is **not** a drop-in for `npx react-devtools` / the Chrome extension
//! (those need `react-devtools-core` + Int32 `operations` on `:8097`). See
//! `docs/DEVTOOLS.md`.
//!
//! ## `egui` feature
//!
//! Shows an egui panel listing the same `node_id` ↔ `Entity` mapping.

#[cfg(feature = "devtools")]
mod ws {
    use std::net::SocketAddr;
    use std::sync::{Arc, Mutex};
    use std::thread;

    use bevy::prelude::*;
    use futures_util::{SinkExt, StreamExt};
    use serde_json::{json, Value};
    use tokio::net::{TcpListener, TcpStream};
    use tokio::sync::broadcast;
    use tokio_tungstenite::{accept_async, tungstenite::Message};

    use crate::js_bevy::JsClientResource;
    use crate::react::{ReactContext, ReactRoot};

    pub const DEVTOOLS_WS_PORT: u16 = 8098;
    pub const DEVTOOLS_WS_ADDR: &str = "127.0.0.1:8098";

    #[derive(Clone)]
    struct SharedSnapshot {
        ecs_map: Arc<Mutex<Value>>,
        tree: Arc<Mutex<Value>>,
        fiber_tree: Arc<Mutex<Value>>,
        operations: Arc<Mutex<Value>>,
    }

    impl SharedSnapshot {
        fn new() -> Self {
            Self {
                ecs_map: Arc::new(Mutex::new(json!({
                    "type": "ecs_map",
                    "roots": []
                }))),
                tree: Arc::new(Mutex::new(json!({
                    "type": "tree",
                    "roots": []
                }))),
                fiber_tree: Arc::new(Mutex::new(json!({
                    "type": "fiber_tree",
                    "roots": []
                }))),
                operations: Arc::new(Mutex::new(json!({
                    "event": "operations",
                    "payload": { "roots": [] }
                }))),
            }
        }
    }

    /// Handle to the DevTools broadcast bus (Bevy resource).
    #[derive(Resource, Clone)]
    pub struct ReactDevToolsBridge {
        tx: broadcast::Sender<String>,
        snapshot: SharedSnapshot,
    }

    impl ReactDevToolsBridge {
        fn publish(&self, msg: &str) {
            let _ = self.tx.send(msg.to_string());
        }
    }

    /// Plugin: WebSocket debug bridge on port 8098.
    pub struct ReactDevToolsPlugin;

    impl Plugin for ReactDevToolsPlugin {
        fn build(&self, app: &mut App) {
            let (tx, _) = broadcast::channel::<String>(64);
            let snapshot = SharedSnapshot::new();
            let bridge = ReactDevToolsBridge {
                tx: tx.clone(),
                snapshot: snapshot.clone(),
            };

            spawn_devtools_server(tx, snapshot);

            app.insert_resource(bridge)
                .add_systems(Update, (publish_ecs_map, request_js_tree_dump));

            log::info!(
                "[bevy-react] DevTools bridge listening on ws://{DEVTOOLS_WS_ADDR}"
            );
        }
    }

    fn spawn_devtools_server(tx: broadcast::Sender<String>, snapshot: SharedSnapshot) {
        thread::Builder::new()
            .name("bevy-react-devtools".into())
            .spawn(move || {
                let rt = match tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(e) => {
                        log::error!("[bevy-react] DevTools runtime failed: {e}");
                        return;
                    }
                };
                rt.block_on(async move {
                    if let Err(e) = run_server(tx, snapshot).await {
                        log::error!("[bevy-react] DevTools server error: {e}");
                    }
                });
            })
            .expect("spawn bevy-react-devtools thread");
    }

    async fn run_server(
        tx: broadcast::Sender<String>,
        snapshot: SharedSnapshot,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let addr: SocketAddr = DEVTOOLS_WS_ADDR.parse()?;
        let listener = TcpListener::bind(addr).await?;
        log::info!("[bevy-react] DevTools WS bound to {addr}");

        loop {
            let (stream, peer) = listener.accept().await?;
            log::info!("[bevy-react] DevTools client connected: {peer}");
            let tx = tx.clone();
            let snapshot = snapshot.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_client(stream, tx, snapshot).await {
                    log::debug!("[bevy-react] DevTools client {peer} ended: {e}");
                }
            });
        }
    }

    async fn handle_client(
        stream: TcpStream,
        tx: broadcast::Sender<String>,
        snapshot: SharedSnapshot,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let ws = accept_async(stream).await?;
        let (mut write, mut read) = ws.split();
        let mut rx = tx.subscribe();

        let hello = json!({
            "type": "hello",
            "version": 2,
            "renderer": "bevy-react",
            "port": DEVTOOLS_WS_PORT,
            "protocol": "bevy-react-devtools-v2",
            "supports": ["ecs_map", "tree", "fiber_tree", "rdt_event_envelopes"],
            "note": "RDT-shaped events + fiber JSON on :8098 — not full react-devtools-core / :8097. See docs/DEVTOOLS.md."
        });
        write
            .send(Message::Text(hello.to_string().into()))
            .await?;

        for key in ["ecs_map", "tree", "fiber_tree", "operations"] {
            let text = match key {
                "ecs_map" => snapshot.ecs_map.lock().ok().map(|g| g.to_string()),
                "tree" => snapshot.tree.lock().ok().map(|g| g.to_string()),
                "fiber_tree" => snapshot.fiber_tree.lock().ok().map(|g| g.to_string()),
                "operations" => snapshot.operations.lock().ok().map(|g| g.to_string()),
                _ => None,
            };
            if let Some(text) = text {
                write.send(Message::Text(text.into())).await?;
            }
        }

        loop {
            tokio::select! {
                msg = rx.recv() => {
                    match msg {
                        Ok(text) => {
                            write.send(Message::Text(text.into())).await?;
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
                incoming = read.next() => {
                    match incoming {
                        Some(Ok(Message::Text(text))) => {
                            on_client_message(&text, &snapshot, &tx);
                        }
                        Some(Ok(Message::Ping(p))) => {
                            write.send(Message::Pong(p)).await?;
                        }
                        Some(Ok(Message::Close(_))) | None => break,
                        Some(Ok(_)) => {}
                        Some(Err(e)) => return Err(e.into()),
                    }
                }
            }
        }
        Ok(())
    }

    fn on_client_message(
        text: &str,
        snapshot: &SharedSnapshot,
        tx: &broadcast::Sender<String>,
    ) {
        let Ok(value) = serde_json::from_str::<Value>(text) else {
            return;
        };
        let typ = value.get("type").and_then(|t| t.as_str()).unwrap_or("");
        let event = value.get("event").and_then(|t| t.as_str()).unwrap_or("");
        match (typ, event) {
            ("tree", _) => {
                if let Ok(mut tree) = snapshot.tree.lock() {
                    *tree = value.clone();
                }
                let _ = tx.send(value.to_string());
            }
            ("fiber_tree", _) => {
                if let Ok(mut fiber) = snapshot.fiber_tree.lock() {
                    *fiber = value.clone();
                }
                let _ = tx.send(value.to_string());
            }
            (_, "operations") => {
                if let Ok(mut ops) = snapshot.operations.lock() {
                    *ops = value.clone();
                }
                let _ = tx.send(value.to_string());
            }
            (_, "backendVersion" | "bridgeProtocol" | "rendererAttached") => {
                // Handshake / meta — broadcast to other clients, do not cache as tree.
                let _ = tx.send(value.to_string());
            }
            ("request_dump" | "ping", _) | (_, "request_dump" | "ping") => {
                for slot in [
                    &snapshot.ecs_map,
                    &snapshot.tree,
                    &snapshot.fiber_tree,
                    &snapshot.operations,
                ] {
                    if let Ok(guard) = slot.lock() {
                        let _ = tx.send(guard.to_string());
                    }
                }
            }
            _ => {}
        }
    }

    fn publish_ecs_map(
        bridge: Res<ReactDevToolsBridge>,
        roots: Query<(&ReactRoot, &ReactContext)>,
        mut last_fp: Local<String>,
    ) {
        let mut root_payloads = Vec::new();
        let mut fingerprint = String::new();

        for (root, ctx) in roots.iter() {
            let mut nodes: Vec<Value> = ctx
                .nodes
                .iter()
                .map(|(node_id, entity)| {
                    fingerprint.push_str(&format!("{node_id}:{entity};"));
                    json!({
                        "nodeId": *node_id,
                        "entity": format!("{entity}"),
                        "index": entity.index(),
                        "generation": format!("{:?}", entity.generation()),
                    })
                })
                .collect();
            nodes.sort_by_key(|n| n["nodeId"].as_u64().unwrap_or(0));

            fingerprint.push_str(&root.id);
            fingerprint.push('|');

            root_payloads.push(json!({
                "rootId": root.id,
                "rootEntity": ctx.root.map(|e| format!("{e}")),
                "nodeCount": ctx.nodes.len(),
                "nodes": nodes,
            }));
        }

        if *last_fp == fingerprint {
            return;
        }
        *last_fp = fingerprint;

        let msg = json!({
            "type": "ecs_map",
            "roots": root_payloads,
        });

        if let Ok(mut ecs) = bridge.snapshot.ecs_map.lock() {
            *ecs = msg.clone();
        }
        bridge.publish(&msg.to_string());
    }

    /// Ask the JS side to refresh its DevTools dump (best-effort).
    fn request_js_tree_dump(
        time: Res<Time>,
        mut last: Local<f32>,
        js: Option<Res<JsClientResource>>,
    ) {
        let now = time.elapsed_secs();
        if now - *last < 2.0 {
            return;
        }
        *last = now;

        let Some(js) = js else {
            return;
        };

        js.execute(
            r#"
            (function () {
              try {
                var api = globalThis.__bevyReactDevTools;
                if (!api) return;
                if (typeof api.dump === 'function') {
                  globalThis.__bevyReactDevToolsLastDump = api.dump();
                }
                if (typeof api.dumpFibers === 'function') {
                  globalThis.__bevyReactDevToolsLastFibers = api.dumpFibers();
                }
              } catch (e) {
                console.warn('[bevy-react] DevTools JS dump failed', e);
              }
            })()
            "#,
        );
    }
}

#[cfg(feature = "devtools")]
pub use ws::{ReactDevToolsBridge, ReactDevToolsPlugin, DEVTOOLS_WS_ADDR, DEVTOOLS_WS_PORT};

/// egui panel: React `node_id` ↔ Bevy `Entity`.
#[cfg(feature = "egui")]
mod inspector {
    use bevy::prelude::*;
    use bevy_egui::{egui, EguiContexts, EguiPlugin};

    use crate::react::{ReactContext, ReactRoot};

    /// Plugin: "React Nodes" egui window over `ReactContext.nodes`.
    pub struct ReactNodeInspectorPlugin;

    impl Plugin for ReactNodeInspectorPlugin {
        fn build(&self, app: &mut App) {
            if !app.is_plugin_added::<EguiPlugin>() {
                app.add_plugins(EguiPlugin::default());
            }
            app.add_systems(Update, react_node_inspector_ui);
        }
    }

    fn react_node_inspector_ui(
        mut contexts: EguiContexts,
        roots: Query<(&ReactRoot, &ReactContext)>,
    ) {
        let Ok(ctx) = contexts.ctx_mut() else {
            return;
        };

        egui::Window::new("React Nodes")
            .default_width(360.0)
            .show(ctx, |ui| {
                ui.label("node_id ↔ Entity (from ReactContext.nodes)");
                ui.separator();

                let mut any = false;
                for (root, react_ctx) in roots.iter() {
                    any = true;
                    egui::CollapsingHeader::new(format!("root: {}", root.id))
                        .default_open(true)
                        .show(ui, |ui| {
                            if let Some(root_entity) = react_ctx.root {
                                ui.label(format!("ReactRoot entity: {root_entity}"));
                            }
                            ui.label(format!("{} nodes", react_ctx.nodes.len()));
                            ui.separator();

                            let mut rows: Vec<_> = react_ctx.nodes.iter().collect();
                            rows.sort_by_key(|(id, _)| *id);

                            egui::ScrollArea::vertical()
                                .max_height(400.0)
                                .show(ui, |ui| {
                                    egui::Grid::new(format!("react_nodes_{}", root.id))
                                        .num_columns(2)
                                        .striped(true)
                                        .show(ui, |ui| {
                                            ui.strong("node_id");
                                            ui.strong("Entity");
                                            ui.end_row();
                                            for (node_id, entity) in rows {
                                                ui.label(format!("{node_id}"));
                                                ui.monospace(format!("{entity}"));
                                                ui.end_row();
                                            }
                                        });
                                });
                        });
                }

                if !any {
                    ui.weak("No ReactContext roots yet.");
                }
            });
    }
}

#[cfg(feature = "egui")]
pub use inspector::ReactNodeInspectorPlugin;
