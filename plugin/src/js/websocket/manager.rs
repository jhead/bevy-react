//! WebSocket Implementation for Boa JS Engine
//!
//! Provides a WebSocket shim that delegates to Rust's tokio-tungstenite.
//! This enables Vite HMR and other WebSocket-dependent features.
//!
//! Events are pushed from Rust to JS via JsEngineClient::execute(),
//! similar to how input events are dispatched.

use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

use crate::js::JsEngineClient;

/// WebSocket ready states (matching browser API)
pub const WS_CONNECTING: u32 = 0;
pub const WS_OPEN: u32 = 1;
pub const WS_CLOSING: u32 = 2;
pub const WS_CLOSED: u32 = 3;

/// A handle to send messages to a WebSocket connection
#[derive(Clone)]
struct WebSocketHandle {
    sender: mpsc::UnboundedSender<String>,
    ready_state: Arc<AtomicU32>,
}

/// Manages all WebSocket connections
pub struct WebSocketManager {
    client: JsEngineClient,
    connections: Mutex<HashMap<u32, WebSocketHandle>>,
    next_id: AtomicU32,
}

impl WebSocketManager {
    pub fn new(client: JsEngineClient) -> Self {
        Self {
            client,
            connections: Mutex::new(HashMap::new()),
            next_id: AtomicU32::new(1),
        }
    }

    /// Push an event to JS by executing code
    fn dispatch_event(client: &JsEngineClient, id: u32, event_type: &str, data: &str) {
        log::info!("[WebSocket {}] Dispatching {} event to JS", id, event_type);
        let script = format!("__ws_dispatch_event({}, '{}', {});", id, event_type, data);
        log::debug!(
            "[WebSocket {}] Executing: {}",
            id,
            &script[..script.len().min(200)]
        );
        client.execute(script);
    }

    /// Connect to a WebSocket URL, returns the connection ID immediately
    pub fn connect(&self, url: String) -> u32 {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        let ready_state = Arc::new(AtomicU32::new(WS_CONNECTING));

        let handle = WebSocketHandle {
            sender: tx,
            ready_state: ready_state.clone(),
        };

        // Store the handle
        {
            let mut connections = self.connections.lock().unwrap();
            connections.insert(id, handle);
        }

        let url_clone = url.clone();
        let ready_state_clone = ready_state.clone();
        
        let client = self.client.clone();

        // Spawn on a separate thread with its own tokio runtime
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .worker_threads(2)
                .build()
                .expect("Failed to create WebSocket runtime");

            rt.block_on(async move {
                log::info!("[WebSocket {}] Connecting to {}", id, url_clone);

                // Parse URL to get host and port
                let url = match url::Url::parse(&url_clone) {
                    Ok(u) => u,
                    Err(e) => {
                        log::error!("[WebSocket {}] Invalid URL: {}", id, e);
                        ready_state_clone.store(WS_CLOSED, Ordering::SeqCst);
                        WebSocketManager::dispatch_event(
                            &client,
                            id,
                            "error",
                            &format!(
                                "{{ \"message\": \"Invalid URL: {}\" }}",
                                escape_json_string(&e.to_string())
                            ),
                        );
                        WebSocketManager::dispatch_event(
                            &client,
                            id,
                            "close",
                            "{ \"code\": 1006, \"reason\": \"Invalid URL\" }",
                        );
                        return;
                    }
                };

                let host = url.host_str().unwrap_or("localhost");
                let port = url
                    .port()
                    .unwrap_or(if url.scheme() == "wss" { 443 } else { 80 });
                let addr = format!("{}:{}", host, port);

                log::info!("[WebSocket {}] Connecting TCP to {}", id, addr);

                // Establish TCP connection
                let tcp_stream = match TcpStream::connect(&addr).await {
                    Ok(stream) => {
                        log::info!("[WebSocket {}] TCP connected", id);
                        stream
                    }
                    Err(e) => {
                        log::error!("[WebSocket {}] TCP connection failed: {}", id, e);
                        ready_state_clone.store(WS_CLOSED, Ordering::SeqCst);
                        WebSocketManager::dispatch_event(
                            &client,
                            id,
                            "error",
                            &format!(
                                "{{ \"message\": \"TCP connection failed: {}\" }}",
                                escape_json_string(&e.to_string())
                            ),
                        );
                        WebSocketManager::dispatch_event(
                            &client,
                            id,
                            "close",
                            "{ \"code\": 1006, \"reason\": \"Connection failed\" }",
                        );
                        return;
                    }
                };

                // Build WebSocket request
                let mut request = match url_clone.as_str().into_client_request() {
                    Ok(req) => req,
                    Err(e) => {
                        log::error!("[WebSocket {}] Failed to create request: {}", id, e);
                        ready_state_clone.store(WS_CLOSED, Ordering::SeqCst);
                        WebSocketManager::dispatch_event(&client,
                            id,
                            "error",
                            &format!(
                                "{{ \"message\": \"Invalid request: {}\" }}",
                                escape_json_string(&e.to_string())
                            ),
                        );
                        WebSocketManager::dispatch_event(
                            &client,
                            id,
                            "close",
                            "{ \"code\": 1006, \"reason\": \"Invalid request\" }",
                        );
                        return;
                    }
                };

                // Add Origin header for browser-like behavior
                request
                    .headers_mut()
                    .insert("Origin", "http://localhost:5173".parse().unwrap());

                // Add Sec-WebSocket-Protocol header which Vite HMR expects
                request
                    .headers_mut()
                    .insert("Sec-WebSocket-Protocol", "vite-hmr".parse().unwrap());

                log::info!("[WebSocket {}] Performing WebSocket handshake", id);

                // Perform WebSocket handshake
                let ws_stream = match tokio_tungstenite::client_async(request, tcp_stream).await {
                    Ok((stream, response)) => {
                        log::info!(
                            "[WebSocket {}] Connected successfully (status: {})",
                            id,
                            response.status()
                        );
                        stream
                    }
                    Err(e) => {
                        log::error!("[WebSocket {}] Handshake failed: {}", id, e);
                        ready_state_clone.store(WS_CLOSED, Ordering::SeqCst);
                        WebSocketManager::dispatch_event(
                            &client,
                            id,
                            "error",
                            &format!(
                                "{{ \"message\": \"Handshake failed: {}\" }}",
                                escape_json_string(&e.to_string())
                            ),
                        );
                        WebSocketManager::dispatch_event(
                            &client,
                            id,
                            "close",
                            "{ \"code\": 1006, \"reason\": \"Handshake failed\" }",
                        );
                        return;
                    }
                };

                // Connection succeeded - update state and dispatch open event
                ready_state_clone.store(WS_OPEN, Ordering::SeqCst);
                WebSocketManager::dispatch_event(&client, id, "open", "{}");

                let (mut write, mut read) = ws_stream.split();

                // Spawn task to forward outgoing messages
                let ready_state_for_send = ready_state_clone.clone();
                let send_task = tokio::spawn(async move {
                    while let Some(msg) = rx.recv().await {
                        if ready_state_for_send.load(Ordering::SeqCst) != WS_OPEN {
                            break;
                        }
                        if let Err(e) = write.send(Message::Text(msg.into())).await {
                            log::error!("[WebSocket {}] Send error: {}", id, e);
                            break;
                        }
                    }
                });

                // Read incoming messages and dispatch to JS
                while let Some(msg_result) = read.next().await {
                    match msg_result {
                        Ok(Message::Text(text)) => {
                            log::debug!(
                                "[WebSocket {}] Received: {}",
                                id,
                                &text[..text.len().min(100)]
                            );
                            WebSocketManager::dispatch_event(
                                &client,
                                id,
                                "message",
                                &format!("{{ \"data\": \"{}\" }}", escape_json_string(&text)),
                            );
                        }
                        Ok(Message::Binary(data)) => {
                            log::debug!(
                                "[WebSocket {}] Received binary ({} bytes)",
                                id,
                                data.len()
                            );
                            // Skip binary for now
                        }
                        Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => {
                            // Handled by tungstenite
                        }
                        Ok(Message::Close(frame)) => {
                            let (code, reason) = frame
                                .map(|f| (f.code.into(), f.reason.to_string()))
                                .unwrap_or((1000, String::new()));
                            log::info!("[WebSocket {}] Received close: {} {}", id, code, reason);
                            ready_state_clone.store(WS_CLOSED, Ordering::SeqCst);
                            WebSocketManager::dispatch_event(
                                &client,
                                id,
                                "close",
                                &format!(
                                    "{{ \"code\": {}, \"reason\": \"{}\" }}",
                                    code,
                                    escape_json_string(&reason)
                                ),
                            );
                            break;
                        }
                        Ok(Message::Frame(_)) => {}
                        Err(e) => {
                            log::error!("[WebSocket {}] Read error: {}", id, e);
                            ready_state_clone.store(WS_CLOSED, Ordering::SeqCst);
                            WebSocketManager::dispatch_event(
                                &client,
                                id,
                                "error",
                                &format!(
                                    "{{ \"message\": \"{}\" }}",
                                    escape_json_string(&e.to_string())
                                ),
                            );
                            WebSocketManager::dispatch_event(
                                &client,
                                id,
                                "close",
                                "{ \"code\": 1006, \"reason\": \"Connection error\" }",
                            );
                            break;
                        }
                    }
                }

                send_task.abort();
                log::info!("[WebSocket {}] Connection ended", id);
            });
        });

        id
    }

    /// Send a message on a WebSocket connection
    pub fn send(&self, id: u32, data: String) -> Result<(), String> {
        let connections = self.connections.lock().unwrap();
        if let Some(handle) = connections.get(&id) {
            if handle.ready_state.load(Ordering::SeqCst) != WS_OPEN {
                return Err("WebSocket is not open".to_string());
            }
            handle
                .sender
                .send(data)
                .map_err(|e| format!("Failed to send: {}", e))
        } else {
            Err("WebSocket not found".to_string())
        }
    }

    /// Close a WebSocket connection
    pub fn close(&self, id: u32, _code: u16, _reason: String) {
        let mut connections = self.connections.lock().unwrap();
        if let Some(handle) = connections.remove(&id) {
            handle.ready_state.store(WS_CLOSING, Ordering::SeqCst);
            log::info!("[WebSocket {}] Closing", id);
        }
    }

    /// Get the ready state of a connection
    pub fn ready_state(&self, id: u32) -> u32 {
        let connections = self.connections.lock().unwrap();
        connections
            .get(&id)
            .map(|h| h.ready_state.load(Ordering::SeqCst))
            .unwrap_or(WS_CLOSED)
    }
}

/// Escape a string for JSON
fn escape_json_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c if c.is_control() => {
                result.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => result.push(c),
        }
    }
    result
}
