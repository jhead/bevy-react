//! WebSocket Implementation for Boa JS Engine
//!
//! Provides a WebSocket shim that delegates to Rust's tokio-tungstenite.
//! This enables Vite HMR and other WebSocket-dependent features.
//!
//! Events are pushed from Rust to JS via JsEngineClient::execute(),
//! similar to how input events are dispatched.

use boa_engine::{Context, JsString, JsValue, NativeFunction};
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;

use crate::js::{JsCallback, JsEngineClient};

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

/// Global WebSocket manager instance
static WEBSOCKET_MANAGER: once_cell::sync::Lazy<WebSocketManager> =
    once_cell::sync::Lazy::new(WebSocketManager::new);

/// Global JS client for pushing events - set during initialization
static JS_CLIENT: once_cell::sync::OnceCell<JsEngineClient> = once_cell::sync::OnceCell::new();

/// Manages all WebSocket connections
pub struct WebSocketManager {
    connections: Mutex<HashMap<u32, WebSocketHandle>>,
    next_id: AtomicU32,
}

impl WebSocketManager {
    pub fn new() -> Self {
        Self {
            connections: Mutex::new(HashMap::new()),
            next_id: AtomicU32::new(1),
        }
    }

    /// Get the global manager instance
    pub fn global() -> &'static WebSocketManager {
        &WEBSOCKET_MANAGER
    }

    /// Set the JS client for pushing events
    pub fn set_js_client(client: JsEngineClient) {
        let _ = JS_CLIENT.set(client);
    }

    /// Get the JS client
    fn js_client() -> Option<&'static JsEngineClient> {
        JS_CLIENT.get()
    }

    /// Push an event to JS by executing code
    fn dispatch_event(id: u32, event_type: &str, data: &str) {
        log::info!("[WebSocket {}] Dispatching {} event to JS", id, event_type);
        if let Some(client) = Self::js_client() {
            let script = format!(
                "__ws_dispatch_event({}, '{}', {});",
                id, event_type, data
            );
            log::debug!("[WebSocket {}] Executing: {}", id, &script[..script.len().min(200)]);
            client.execute(script);
        } else {
            log::warn!("[WebSocket {}] No JS client available to dispatch {} event", id, event_type);
        }
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
                        Self::dispatch_event(id, "error", &format!(
                            "{{ \"message\": \"Invalid URL: {}\" }}", 
                            escape_json_string(&e.to_string())
                        ));
                        Self::dispatch_event(id, "close", "{ \"code\": 1006, \"reason\": \"Invalid URL\" }");
                        return;
                    }
                };

                let host = url.host_str().unwrap_or("localhost");
                let port = url.port().unwrap_or(if url.scheme() == "wss" { 443 } else { 80 });
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
                        Self::dispatch_event(id, "error", &format!(
                            "{{ \"message\": \"TCP connection failed: {}\" }}", 
                            escape_json_string(&e.to_string())
                        ));
                        Self::dispatch_event(id, "close", "{ \"code\": 1006, \"reason\": \"Connection failed\" }");
                        return;
                    }
                };

                // Build WebSocket request
                let mut request = match url_clone.as_str().into_client_request() {
                    Ok(req) => req,
                    Err(e) => {
                        log::error!("[WebSocket {}] Failed to create request: {}", id, e);
                        ready_state_clone.store(WS_CLOSED, Ordering::SeqCst);
                        Self::dispatch_event(id, "error", &format!(
                            "{{ \"message\": \"Invalid request: {}\" }}", 
                            escape_json_string(&e.to_string())
                        ));
                        Self::dispatch_event(id, "close", "{ \"code\": 1006, \"reason\": \"Invalid request\" }");
                        return;
                    }
                };

                // Add Origin header for browser-like behavior
                request.headers_mut().insert(
                    "Origin",
                    "http://localhost:5173".parse().unwrap(),
                );

                // Add Sec-WebSocket-Protocol header which Vite HMR expects
                request.headers_mut().insert(
                    "Sec-WebSocket-Protocol",
                    "vite-hmr".parse().unwrap(),
                );

                log::info!("[WebSocket {}] Performing WebSocket handshake", id);

                // Perform WebSocket handshake
                let ws_stream = match tokio_tungstenite::client_async(request, tcp_stream).await {
                    Ok((stream, response)) => {
                        log::info!("[WebSocket {}] Connected successfully (status: {})", id, response.status());
                        stream
                    }
                    Err(e) => {
                        log::error!("[WebSocket {}] Handshake failed: {}", id, e);
                        ready_state_clone.store(WS_CLOSED, Ordering::SeqCst);
                        Self::dispatch_event(id, "error", &format!(
                            "{{ \"message\": \"Handshake failed: {}\" }}", 
                            escape_json_string(&e.to_string())
                        ));
                        Self::dispatch_event(id, "close", "{ \"code\": 1006, \"reason\": \"Handshake failed\" }");
                        return;
                    }
                };

                // Connection succeeded - update state and dispatch open event
                ready_state_clone.store(WS_OPEN, Ordering::SeqCst);
                Self::dispatch_event(id, "open", "{}");

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
                            Self::dispatch_event(id, "message", &format!(
                                "{{ \"data\": \"{}\" }}", 
                                escape_json_string(&text)
                            ));
                        }
                        Ok(Message::Binary(data)) => {
                            log::debug!("[WebSocket {}] Received binary ({} bytes)", id, data.len());
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
                            Self::dispatch_event(id, "close", &format!(
                                "{{ \"code\": {}, \"reason\": \"{}\" }}", 
                                code, escape_json_string(&reason)
                            ));
                            break;
                        }
                        Ok(Message::Frame(_)) => {}
                        Err(e) => {
                            log::error!("[WebSocket {}] Read error: {}", id, e);
                            ready_state_clone.store(WS_CLOSED, Ordering::SeqCst);
                            Self::dispatch_event(id, "error", &format!(
                                "{{ \"message\": \"{}\" }}", 
                                escape_json_string(&e.to_string())
                            ));
                            Self::dispatch_event(id, "close", "{ \"code\": 1006, \"reason\": \"Connection error\" }");
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

/// JS callback that registers WebSocket native functions
pub struct WebSocketJsCallback {
    client: JsEngineClient,
}

impl WebSocketJsCallback {
    pub fn new(client: JsEngineClient) -> Arc<Self> {
        Arc::new(Self { client })
    }
}

impl JsCallback for WebSocketJsCallback {
    fn register(&self, context: &mut Context) {
        // Store the JS client for event dispatch
        WebSocketManager::set_js_client(self.client.clone());
        
        register_websocket_functions(context);
        register_websocket_shim(context);
    }
}

/// Register WebSocket native functions
fn register_websocket_functions(context: &mut Context) {
    // __ws_connect(url: string) -> number
    context
        .register_global_callable(
            JsString::from("__ws_connect"),
            1,
            NativeFunction::from_copy_closure(
                |_this: &JsValue, args: &[JsValue], _ctx: &mut Context| {
                    let url = args
                        .first()
                        .and_then(|v| v.as_string())
                        .map(|s| s.to_std_string_escaped())
                        .unwrap_or_default();

                    log::info!("[WebSocket Native] connect({})", url);
                    let id = WebSocketManager::global().connect(url);
                    Ok(JsValue::from(id))
                },
            ),
        )
        .expect("Failed to register __ws_connect");

    // __ws_send(id: number, data: string) -> void
    context
        .register_global_callable(
            JsString::from("__ws_send"),
            2,
            NativeFunction::from_copy_closure(
                |_this: &JsValue, args: &[JsValue], ctx: &mut Context| {
                    let id = args
                        .first()
                        .and_then(|v| v.to_u32(ctx).ok())
                        .unwrap_or(0);
                    let data = args
                        .get(1)
                        .and_then(|v| v.as_string())
                        .map(|s| s.to_std_string_escaped())
                        .unwrap_or_default();

                    log::debug!("[WebSocket Native] send({}, {} bytes)", id, data.len());
                    if let Err(e) = WebSocketManager::global().send(id, data) {
                        log::error!("[WebSocket Native] send error: {}", e);
                    }
                    Ok(JsValue::undefined())
                },
            ),
        )
        .expect("Failed to register __ws_send");

    // __ws_close(id: number, code: number, reason: string) -> void
    context
        .register_global_callable(
            JsString::from("__ws_close"),
            3,
            NativeFunction::from_copy_closure(
                |_this: &JsValue, args: &[JsValue], ctx: &mut Context| {
                    let id = args
                        .first()
                        .and_then(|v| v.to_u32(ctx).ok())
                        .unwrap_or(0);
                    let code = args
                        .get(1)
                        .and_then(|v| v.to_u32(ctx).ok())
                        .unwrap_or(1000) as u16;
                    let reason = args
                        .get(2)
                        .and_then(|v| v.as_string())
                        .map(|s| s.to_std_string_escaped())
                        .unwrap_or_default();

                    log::info!("[WebSocket Native] close({}, {}, {})", id, code, reason);
                    WebSocketManager::global().close(id, code, reason);
                    Ok(JsValue::undefined())
                },
            ),
        )
        .expect("Failed to register __ws_close");

    // __ws_ready_state(id: number) -> number
    context
        .register_global_callable(
            JsString::from("__ws_ready_state"),
            1,
            NativeFunction::from_copy_closure(
                |_this: &JsValue, args: &[JsValue], ctx: &mut Context| {
                    let id = args
                        .first()
                        .and_then(|v| v.to_u32(ctx).ok())
                        .unwrap_or(0);

                    let state = WebSocketManager::global().ready_state(id);
                    Ok(JsValue::from(state))
                },
            ),
        )
        .expect("Failed to register __ws_ready_state");

    log::info!("Registered WebSocket native functions");
}

/// Register the WebSocket JavaScript shim
fn register_websocket_shim(context: &mut Context) {
    let shim = r#"
(function() {
    // WebSocket registry to track instances by connection ID
    var wsRegistry = {};

    // Event dispatch function called from Rust
    globalThis.__ws_dispatch_event = function(id, eventType, eventData) {
        console.log('[WebSocket JS] Dispatch event:', id, eventType, eventData);
        var ws = wsRegistry[id];
        if (!ws) {
            console.warn('[WebSocket JS] No instance found for id', id, 'event', eventType);
            return;
        }

        var event = Object.assign({ type: eventType, target: ws }, eventData);

        switch (eventType) {
            case 'open':
                ws._readyState = 1; // OPEN
                if (ws.onopen) {
                    try { ws.onopen(event); } catch (e) { console.error('[WebSocket] onopen error:', e); }
                }
                ws.dispatchEvent(event);
                break;

            case 'message':
                if (ws.onmessage) {
                    try { ws.onmessage(event); } catch (e) { console.error('[WebSocket] onmessage error:', e); }
                }
                ws.dispatchEvent(event);
                break;

            case 'close':
                ws._readyState = 3; // CLOSED
                event.wasClean = (event.code === 1000);
                if (ws.onclose) {
                    try { ws.onclose(event); } catch (e) { console.error('[WebSocket] onclose error:', e); }
                }
                ws.dispatchEvent(event);
                delete wsRegistry[id];
                break;

            case 'error':
                if (ws.onerror) {
                    try { ws.onerror(event); } catch (e) { console.error('[WebSocket] onerror error:', e); }
                }
                ws.dispatchEvent(event);
                break;
        }
    };

    // WebSocket constructor
    function WebSocket(url, protocols) {
        console.log('[WebSocket JS] Constructor called with url:', url);
        
        if (!(this instanceof WebSocket)) {
            throw new TypeError("Failed to construct 'WebSocket': Please use the 'new' operator");
        }

        this._url = url;
        this._protocols = protocols || [];
        this._readyState = 0; // CONNECTING
        this._eventListeners = {};
        this.binaryType = 'blob';
        this.bufferedAmount = 0;
        this.extensions = '';
        this.protocol = '';

        // Event handlers
        this.onopen = null;
        this.onmessage = null;
        this.onerror = null;
        this.onclose = null;

        // Connect and register
        console.log('[WebSocket JS] Calling __ws_connect...');
        this._id = __ws_connect(url);
        wsRegistry[this._id] = this;

        console.log('[WebSocket JS] Created connection', this._id, 'to', url);
    }

    // Static constants
    WebSocket.CONNECTING = 0;
    WebSocket.OPEN = 1;
    WebSocket.CLOSING = 2;
    WebSocket.CLOSED = 3;

    // Instance properties
    Object.defineProperties(WebSocket.prototype, {
        url: { get: function() { return this._url; } },
        readyState: { get: function() { return this._readyState; } }
    });

    // Send data
    WebSocket.prototype.send = function(data) {
        if (this._readyState !== 1) {
            throw new Error("Failed to execute 'send' on 'WebSocket': Still in CONNECTING state.");
        }

        var dataStr = (typeof data === 'string') ? data : 
                      (data && typeof data === 'object') ? JSON.stringify(data) : String(data);
        __ws_send(this._id, dataStr);
    };

    // Close connection
    WebSocket.prototype.close = function(code, reason) {
        if (this._readyState === 2 || this._readyState === 3) return;
        this._readyState = 2; // CLOSING
        __ws_close(this._id, code || 1000, reason || '');
    };

    // Event listener methods
    WebSocket.prototype.addEventListener = function(type, listener) {
        if (!this._eventListeners[type]) this._eventListeners[type] = [];
        this._eventListeners[type].push(listener);
    };

    WebSocket.prototype.removeEventListener = function(type, listener) {
        if (!this._eventListeners[type]) return;
        var idx = this._eventListeners[type].indexOf(listener);
        if (idx !== -1) this._eventListeners[type].splice(idx, 1);
    };

    WebSocket.prototype.dispatchEvent = function(event) {
        var listeners = this._eventListeners[event.type];
        if (listeners) {
            for (var i = 0; i < listeners.length; i++) {
                try { listeners[i].call(this, event); } catch (e) { console.error('[WebSocket] listener error:', e); }
            }
        }
    };

    // Register globally
    globalThis.WebSocket = WebSocket;

    console.log('[Shims] WebSocket initialized (push-based events)');
})();
    "#;

    if let Err(e) = context.eval(boa_engine::Source::from_bytes(shim.as_bytes())) {
        log::error!("Failed to register WebSocket shim: {:?}", e);
    }
}
