use boa_engine::{Context, JsError, JsString, JsValue, NativeFunction};

use crate::js::JsEngineExtension;
use crate::js::JsEngineClient;
use crate::js::websocket::manager::WebSocketManager;

pub struct WebSocketExtension;

impl JsEngineExtension for WebSocketExtension {
    fn register(&self, context: &mut Context, client: JsEngineClient) -> Result<(), JsError> {
        let manager = WebSocketManager::new(client);
        register_websocket_functions(context, manager);
        register_websocket_shim(context);
        Ok(())
    }
}

/// Register WebSocket native functions
fn register_websocket_functions(context: &mut Context, manager: WebSocketManager) {
    // Leak the manager to get a 'static reference for Boa
    // TODO: Maybe find a better way to do this without leaking
    let manager = Box::leak(Box::new(manager));

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
                    let id = manager.connect(url);
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
                    if let Err(e) = manager.send(id, data) {
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
                    manager.close(id, code, reason);
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

                    let state = manager.ready_state(id);
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
