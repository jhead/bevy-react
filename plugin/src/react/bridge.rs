//! Rust ↔ React data bridge.
//!
//! Two directions:
//! - **ECS → React:** [`ReactBridge::publish`] pushes JSON on a named channel;
//!   a Bevy system flushes dirty channels into JS via `__react_flush_bridge`.
//! - **JS → Rust:** JS calls `__react_call(name, argsJson)`; handlers registered
//!   with [`ReactBridge::register`] run on the Bevy main thread with `&mut World`.
//!
//! See `docs/BRIDGE.md` for the public API sketch.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use boa_engine::{Context, JsError, JsResult, JsString, JsValue, NativeFunction};
use boa_gc::{Finalize, Trace, empty_trace};
use serde::Serialize;
use serde_json::Value;

/// A pending JS → Rust invocation.
#[derive(Clone, Debug)]
pub struct BridgeCall {
    pub name: String,
    pub args: Value,
}

type BridgeHandler = Arc<dyn Fn(&mut World, Value) -> Value + Send + Sync>;

#[derive(Default)]
struct ReactBridgeInner {
    /// Latest JSON snapshot per channel.
    state: HashMap<String, Value>,
    /// Channels changed since the last flush to JS.
    dirty: HashSet<String>,
    /// Queued `__react_call` invocations.
    calls: VecDeque<BridgeCall>,
    /// Named handlers invoked from [`process_react_bridge_calls`].
    handlers: HashMap<String, BridgeHandler>,
}

/// Thread-safe bridge shared between Bevy systems and Boa native functions.
#[derive(Clone, Default, Finalize, Resource)]
pub struct ReactBridge {
    inner: Arc<Mutex<ReactBridgeInner>>,
}

unsafe impl Trace for ReactBridge {
    empty_trace!();
}

impl ReactBridge {
    pub fn new() -> Self {
        Self::default()
    }

    /// Publish (or replace) JSON state for a named channel.
    ///
    /// Dirty channels are delivered to React on the next
    /// [`flush_react_bridge`](flush_react_bridge) tick.
    pub fn publish(&self, channel: impl Into<String>, value: impl Serialize) {
        let channel = channel.into();
        let Ok(json) = serde_json::to_value(value) else {
            log::error!("ReactBridge::publish failed to serialize channel '{channel}'");
            return;
        };

        if let Ok(mut inner) = self.inner.lock() {
            inner.state.insert(channel.clone(), json);
            inner.dirty.insert(channel);
        }
    }

    /// Register a handler callable from JS as `__react_call(name, argsJson)`.
    ///
    /// Handlers run on the Bevy main thread inside
    /// [`process_react_bridge_calls`] with exclusive [`World`] access.
    /// The return value is currently discarded (fire-and-forget from JS);
    /// use [`publish`](Self::publish) to push results back into React.
    pub fn register<F>(&self, name: impl Into<String>, handler: F)
    where
        F: Fn(&mut World, Value) -> Value + Send + Sync + 'static,
    {
        let name = name.into();
        if let Ok(mut inner) = self.inner.lock() {
            inner.handlers.insert(name, Arc::new(handler));
        }
    }

    /// Remove a previously registered handler.
    pub fn unregister(&self, name: &str) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.handlers.remove(name);
        }
    }

    /// True when at least one channel has unpublished updates.
    pub fn has_pending_state(&self) -> bool {
        self.inner
            .lock()
            .map(|inner| !inner.dirty.is_empty())
            .unwrap_or(false)
    }

    /// True when JS has enqueued calls that have not been processed yet.
    pub fn has_pending_calls(&self) -> bool {
        self.inner
            .lock()
            .map(|inner| !inner.calls.is_empty())
            .unwrap_or(false)
    }

    /// Snapshot of the latest value for a channel (for tests / debugging).
    pub fn get_state(&self, channel: &str) -> Option<Value> {
        self.inner
            .lock()
            .ok()
            .and_then(|inner| inner.state.get(channel).cloned())
    }

    pub(crate) fn enqueue_call(&self, name: String, args: Value) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.calls.push_back(BridgeCall { name, args });
        }
    }

    pub(crate) fn drain_state_updates(&self) -> Vec<(String, Value)> {
        let Ok(mut inner) = self.inner.lock() else {
            return Vec::new();
        };

        let channels: Vec<String> = inner.dirty.drain().collect();
        channels
            .into_iter()
            .filter_map(|channel| {
                let value = inner.state.get(&channel)?.clone();
                Some((channel, value))
            })
            .collect()
    }

    fn drain_calls_and_handlers(&self) -> (Vec<BridgeCall>, HashMap<String, BridgeHandler>) {
        let Ok(mut inner) = self.inner.lock() else {
            return (Vec::new(), HashMap::new());
        };
        let calls = inner.calls.drain(..).collect();
        let handlers = inner.handlers.clone();
        (calls, handlers)
    }
}

/// Fixed script executed on the JS thread to drain dirty bridge state.
pub const FLUSH_BRIDGE_SCRIPT: &str = "__react_flush_bridge();";

/// Run queued `__react_call` handlers with exclusive [`World`] access.
pub fn process_react_bridge_calls(world: &mut World) {
    let Some(bridge) = world.get_resource::<ReactBridge>().cloned() else {
        return;
    };

    let (calls, handlers) = bridge.drain_calls_and_handlers();
    if calls.is_empty() {
        return;
    }

    for call in calls {
        match handlers.get(&call.name) {
            Some(handler) => {
                let _result = handler(world, call.args);
            }
            None => {
                log::warn!(
                    "ReactBridge: no handler registered for '{}'",
                    call.name
                );
            }
        }
    }
}

/// Push dirty channel snapshots into JS via `__react_flush_bridge`.
pub fn flush_react_bridge(
    bridge: Option<Res<ReactBridge>>,
    js_client: Option<Res<crate::js_bevy::JsClientResource>>,
) {
    let Some(bridge) = bridge else {
        return;
    };
    let Some(js_client) = js_client else {
        return;
    };

    if !bridge.has_pending_state() {
        return;
    }

    js_client.execute(FLUSH_BRIDGE_SCRIPT);
}

/// Register bridge native functions on the JS global object.
pub fn register_bridge_functions(
    context: &mut Context,
    bridge: ReactBridge,
) -> Result<(), JsError> {
    // __react_register_bridge_dispatcher(callback) -> void
    context.register_global_callable(
        JsString::from("__react_register_bridge_dispatcher"),
        1,
        NativeFunction::from_copy_closure(
            move |_this: &JsValue, args: &[JsValue], ctx: &mut Context| {
                register_bridge_dispatcher_fn(args, ctx)
            },
        ),
    )?;

    // __react_flush_bridge() -> void
    context.register_global_callable(
        JsString::from("__react_flush_bridge"),
        0,
        NativeFunction::from_copy_closure_with_captures(
            move |_this: &JsValue,
                  _args: &[JsValue],
                  bridge: &ReactBridge,
                  ctx: &mut Context| { flush_bridge_fn(bridge, ctx) },
            bridge.clone(),
        ),
    )?;

    // __react_call(name, args_json) -> void
    context.register_global_callable(
        JsString::from("__react_call"),
        2,
        NativeFunction::from_copy_closure_with_captures(
            move |_this: &JsValue, args: &[JsValue], bridge: &ReactBridge, _ctx: &mut Context| {
                call_fn(args, bridge)
            },
            bridge,
        ),
    )?;

    log::debug!("Registered React bridge native functions");
    Ok(())
}

fn register_bridge_dispatcher_fn(args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let callback = args.first().cloned().unwrap_or(JsValue::undefined());
    if !callback.is_callable() {
        return Err(JsError::from_opaque(JsValue::from(JsString::from(
            "__react_register_bridge_dispatcher expects a function",
        ))));
    }

    let global = ctx.global_object();
    global.set(
        JsString::from("__react_bridge_dispatcher"),
        callback,
        true,
        ctx,
    )?;
    log::debug!("Registered React bridge dispatcher callback");
    Ok(JsValue::undefined())
}

fn flush_bridge_fn(bridge: &ReactBridge, ctx: &mut Context) -> JsResult<JsValue> {
    let updates = bridge.drain_state_updates();
    if updates.is_empty() {
        return Ok(JsValue::undefined());
    }

    let global = ctx.global_object();
    let dispatcher = global.get(JsString::from("__react_bridge_dispatcher"), ctx)?;
    let Some(callable) = dispatcher.as_callable() else {
        log::warn!(
            "__react_flush_bridge: no dispatcher registered ({} updates dropped)",
            updates.len()
        );
        return Ok(JsValue::undefined());
    };

    for (channel, value) in updates {
        let payload = JsValue::from_json(&value, ctx)?;
        let call_args = [
            JsValue::from(JsString::from(channel.as_str())),
            payload,
        ];

        if let Err(err) = callable.call(&JsValue::undefined(), &call_args, ctx) {
            log::error!("Failed to dispatch bridge state for '{channel}': {err:?}");
        }
    }

    Ok(JsValue::undefined())
}

fn call_fn(args: &[JsValue], bridge: &ReactBridge) -> JsResult<JsValue> {
    let name = args
        .first()
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .ok_or_else(|| {
            JsError::from_opaque(JsValue::from(JsString::from(
                "__react_call expects a function name string",
            )))
        })?;

    let args_json = args
        .get(1)
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_else(|| "null".to_string());

    let parsed: Value = serde_json::from_str(&args_json).unwrap_or(Value::Null);
    bridge.enqueue_call(name, parsed);
    Ok(JsValue::undefined())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn publish_marks_channel_dirty_and_drains() {
        let bridge = ReactBridge::new();
        bridge.publish("hud", json!({ "hp": 100 }));
        assert!(bridge.has_pending_state());
        assert_eq!(bridge.get_state("hud"), Some(json!({ "hp": 100 })));

        let updates = bridge.drain_state_updates();
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].0, "hud");
        assert!(!bridge.has_pending_state());
    }

    #[test]
    fn publish_overwrites_snapshot() {
        let bridge = ReactBridge::new();
        bridge.publish("hud", json!({ "hp": 100 }));
        bridge.publish("hud", json!({ "hp": 50 }));
        let updates = bridge.drain_state_updates();
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].1, json!({ "hp": 50 }));
    }

    #[test]
    fn enqueue_and_process_registered_handler() {
        let mut app = App::new();
        app.init_resource::<ReactBridge>();
        app.world_mut()
            .resource::<ReactBridge>()
            .register("add", |world, args| {
                let n = args.as_i64().unwrap_or(0);
                let mut score = world.get_resource_or_insert_with(|| Score(0));
                score.0 += n as i32;
                Value::Null
            });

        app.world_mut()
            .resource::<ReactBridge>()
            .enqueue_call("add".into(), json!(7));

        process_react_bridge_calls(app.world_mut());
        assert_eq!(app.world().resource::<Score>().0, 7);
        assert!(!app.world().resource::<ReactBridge>().has_pending_calls());
    }

    #[derive(Resource)]
    struct Score(i32);
}
