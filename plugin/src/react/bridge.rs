//! Rust ↔ React data bridge.
//!
//! Two directions:
//! - **ECS → React:** [`ReactBridge::publish`] pushes JSON on a named channel;
//!   [`ReactBridge::register_resource_store`] snapshots a [`Resource`] when it
//!   changes; [`ReactBridge::register_query_store`] runs a user closure when
//!   dirty (or each frame). A Bevy system flushes dirty channels into JS via
//!   `__react_flush_bridge`.
//! - **JS → Rust:** JS calls `__react_call(name, argsJson, callId)`; handlers
//!   registered with [`ReactBridge::register`] run on the Bevy main thread with
//!   `&mut World`. Return values resolve the matching JS promise.
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
    pub id: u64,
    pub name: String,
    pub args: Value,
}

/// A completed handler result waiting to be delivered to JS.
#[derive(Clone, Debug)]
pub struct BridgeCallResult {
    pub id: u64,
    pub value: Value,
}

type BridgeHandler = Arc<dyn Fn(&mut World, Value) -> Value + Send + Sync>;

/// Type-erased snapshotter for a registered resource store.
type StoreSnapshotter = Arc<dyn Fn(&World) -> Option<Value> + Send + Sync>;

/// Type-erased query snapshotter (`&mut World` so closures can run queries).
type QuerySnapshotter = Arc<dyn Fn(&mut World) -> Value + Send + Sync>;

struct ResourceStoreEntry {
    channel: String,
    try_snapshot: StoreSnapshotter,
}

struct QueryStoreEntry {
    channel: String,
    snapshot: QuerySnapshotter,
    /// When true, the snapshotter runs every flush (skip publish if unchanged).
    each_frame: bool,
}

#[derive(Default)]
struct ReactBridgeInner {
    /// Latest JSON snapshot per channel.
    state: HashMap<String, Value>,
    /// Channels changed since the last flush to JS.
    dirty: HashSet<String>,
    /// Query-store channels that should snapshot on the next sync.
    query_dirty: HashSet<String>,
    /// Queued `__react_call` invocations.
    calls: VecDeque<BridgeCall>,
    /// Completed call results waiting for the next JS flush.
    call_results: VecDeque<BridgeCallResult>,
    /// Named handlers invoked from [`process_react_bridge_calls`].
    handlers: HashMap<String, BridgeHandler>,
    /// ECS-backed stores flushed by [`sync_registered_resource_stores`].
    resource_stores: Vec<ResourceStoreEntry>,
    /// Query-backed stores flushed by [`sync_registered_query_stores`].
    query_stores: Vec<QueryStoreEntry>,
    /// Monotonic id for JS→Rust calls (used when JS omits an id).
    next_call_id: u64,
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

    /// Register an ECS [`Resource`] as a named store.
    ///
    /// Each frame, [`sync_registered_resource_stores`] snapshots `T` when it is
    /// added/changed (or when the channel has never been published) and feeds
    /// the existing [`publish`](Self::publish) / flush path.
    pub fn register_resource_store<T>(&self, channel: impl Into<String>)
    where
        T: Resource + Serialize + Clone,
    {
        let channel = channel.into();
        let channel_key = channel.clone();
        let bridge = self.clone();

        let try_snapshot: StoreSnapshotter = Arc::new(move |world: &World| {
            let missing = bridge.get_state(&channel_key).is_none();
            let changed =
                world.is_resource_changed::<T>() || world.is_resource_added::<T>();
            if !changed && !missing {
                return None;
            }
            let resource = world.get_resource::<T>()?;
            match serde_json::to_value(resource.clone()) {
                Ok(value) => Some(value),
                Err(err) => {
                    log::error!(
                        "ReactBridge resource store '{channel_key}' failed to serialize: {err}"
                    );
                    None
                }
            }
        });

        if let Ok(mut inner) = self.inner.lock() {
            // Replace an existing registration for the same channel.
            inner
                .resource_stores
                .retain(|entry| entry.channel != channel);
            inner.resource_stores.push(ResourceStoreEntry {
                channel,
                try_snapshot,
            });
        }
    }

    /// Remove a resource store registration (does not clear channel state).
    pub fn unregister_resource_store(&self, channel: &str) {
        if let Ok(mut inner) = self.inner.lock() {
            inner
                .resource_stores
                .retain(|entry| entry.channel != channel);
        }
    }

    /// Register a named query store.
    ///
    /// The closure runs when the channel is dirty ([`mark_query_dirty`]) or has
    /// never been published, then feeds the existing publish / flush path.
    /// Prefer marking dirty from a system that observes `Changed` / `Added`
    /// (and removals) rather than serializing every frame.
    ///
    /// For cheap queries, use [`register_query_store_each_frame`](Self::register_query_store_each_frame).
    pub fn register_query_store<F>(&self, channel: impl Into<String>, query_fn: F)
    where
        F: Fn(&mut World) -> Value + Send + Sync + 'static,
    {
        self.insert_query_store(channel.into(), false, query_fn);
    }

    /// Like [`register_query_store`](Self::register_query_store), but snapshots
    /// every flush. Unchanged JSON is not republished.
    pub fn register_query_store_each_frame<F>(&self, channel: impl Into<String>, query_fn: F)
    where
        F: Fn(&mut World) -> Value + Send + Sync + 'static,
    {
        self.insert_query_store(channel.into(), true, query_fn);
    }

    fn insert_query_store<F>(&self, channel: String, each_frame: bool, query_fn: F)
    where
        F: Fn(&mut World) -> Value + Send + Sync + 'static,
    {
        if let Ok(mut inner) = self.inner.lock() {
            inner
                .query_stores
                .retain(|entry| entry.channel != channel);
            inner.query_dirty.insert(channel.clone());
            inner.query_stores.push(QueryStoreEntry {
                channel,
                snapshot: Arc::new(query_fn),
                each_frame,
            });
        }
    }

    /// Mark a query store dirty so the next flush re-runs its snapshotter.
    pub fn mark_query_dirty(&self, channel: &str) {
        if let Ok(mut inner) = self.inner.lock() {
            if inner.query_stores.iter().any(|e| e.channel == channel) {
                inner.query_dirty.insert(channel.to_string());
            }
        }
    }

    /// Remove a query store registration (does not clear channel state).
    pub fn unregister_query_store(&self, channel: &str) {
        if let Ok(mut inner) = self.inner.lock() {
            inner
                .query_stores
                .retain(|entry| entry.channel != channel);
            inner.query_dirty.remove(channel);
        }
    }

    /// Register a handler callable from JS as `__react_call(name, argsJson, callId)`.
    ///
    /// Handlers run on the Bevy main thread inside
    /// [`process_react_bridge_calls`] with exclusive [`World`] access.
    /// The returned [`Value`] is delivered to JS and resolves the matching
    /// `callNative` promise.
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

    /// True when handler results are waiting to be flushed to JS.
    pub fn has_pending_call_results(&self) -> bool {
        self.inner
            .lock()
            .map(|inner| !inner.call_results.is_empty())
            .unwrap_or(false)
    }

    /// Snapshot of the latest value for a channel (for tests / debugging).
    pub fn get_state(&self, channel: &str) -> Option<Value> {
        self.inner
            .lock()
            .ok()
            .and_then(|inner| inner.state.get(channel).cloned())
    }

    pub(crate) fn enqueue_call(&self, name: String, args: Value, id: Option<u64>) {
        if let Ok(mut inner) = self.inner.lock() {
            let id = id.unwrap_or_else(|| {
                inner.next_call_id = inner.next_call_id.saturating_add(1);
                inner.next_call_id
            });
            inner.calls.push_back(BridgeCall { id, name, args });
        }
    }

    pub(crate) fn enqueue_call_result(&self, id: u64, value: Value) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.call_results.push_back(BridgeCallResult { id, value });
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

    pub(crate) fn drain_call_results(&self) -> Vec<BridgeCallResult> {
        let Ok(mut inner) = self.inner.lock() else {
            return Vec::new();
        };
        inner.call_results.drain(..).collect()
    }

    fn clone_resource_stores(&self) -> Vec<(String, StoreSnapshotter)> {
        let Ok(inner) = self.inner.lock() else {
            return Vec::new();
        };
        inner
            .resource_stores
            .iter()
            .map(|entry| (entry.channel.clone(), Arc::clone(&entry.try_snapshot)))
            .collect()
    }

    /// Snapshot of query stores that should run this frame, plus whether each
    /// is `each_frame`. Consumes matching `query_dirty` entries.
    fn take_query_stores_to_sync(&self) -> Vec<(String, QuerySnapshotter, bool)> {
        let Ok(mut inner) = self.inner.lock() else {
            return Vec::new();
        };

        let mut out = Vec::new();
        for entry in &inner.query_stores {
            let missing = !inner.state.contains_key(&entry.channel);
            let marked = inner.query_dirty.contains(&entry.channel);
            if entry.each_frame || missing || marked {
                out.push((
                    entry.channel.clone(),
                    Arc::clone(&entry.snapshot),
                    entry.each_frame,
                ));
            }
        }
        for (channel, _, _) in &out {
            inner.query_dirty.remove(channel);
        }
        out
    }

    /// Publish only when the value differs from the current channel snapshot.
    fn publish_if_changed(&self, channel: String, value: Value) {
        if let Ok(mut inner) = self.inner.lock() {
            let changed = match inner.state.get(&channel) {
                Some(prev) => prev != &value,
                None => true,
            };
            if changed {
                inner.state.insert(channel.clone(), value);
                inner.dirty.insert(channel);
            }
        }
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

/// Fixed script executed on the JS thread to drain dirty bridge state / call results.
pub const FLUSH_BRIDGE_SCRIPT: &str = "__react_flush_bridge();";

/// Snapshot registered resource stores into the publish/dirty path.
///
/// Invoked from [`flush_react_bridge`] so stores land in the same frame flush
/// as manual [`ReactBridge::publish`] updates.
pub fn sync_registered_resource_stores(world: &mut World) {
    let Some(bridge) = world.get_resource::<ReactBridge>().cloned() else {
        return;
    };

    let stores = bridge.clone_resource_stores();
    if stores.is_empty() {
        return;
    }

    for (channel, try_snapshot) in stores {
        if let Some(value) = try_snapshot(world) {
            bridge.publish(channel, value);
        }
    }
}

/// Snapshot registered query stores into the publish/dirty path.
///
/// Runs closures marked via [`ReactBridge::mark_query_dirty`], never-published
/// stores, and `each_frame` stores. Unchanged JSON is not republished.
pub fn sync_registered_query_stores(world: &mut World) {
    let Some(bridge) = world.get_resource::<ReactBridge>().cloned() else {
        return;
    };

    let stores = bridge.take_query_stores_to_sync();
    if stores.is_empty() {
        return;
    }

    for (channel, snapshot, _each_frame) in stores {
        let value = snapshot(world);
        bridge.publish_if_changed(channel, value);
    }
}

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
                let result = handler(world, call.args);
                bridge.enqueue_call_result(call.id, result);
            }
            None => {
                log::warn!(
                    "ReactBridge: no handler registered for '{}'",
                    call.name
                );
                bridge.enqueue_call_result(
                    call.id,
                    serde_json::json!({
                        "error": format!("no handler registered for '{}'", call.name),
                    }),
                );
            }
        }
    }
}

/// Push dirty channel snapshots and call results into JS via `__react_flush_bridge`.
pub fn flush_react_bridge(world: &mut World) {
    sync_registered_resource_stores(world);
    sync_registered_query_stores(world);

    let Some(bridge) = world.get_resource::<ReactBridge>().cloned() else {
        return;
    };
    let Some(js_client) = world.get_resource::<crate::js_bevy::JsClientResource>() else {
        return;
    };

    if !bridge.has_pending_state() && !bridge.has_pending_call_results() {
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

    // __react_register_bridge_call_resolver(callback) -> void
    context.register_global_callable(
        JsString::from("__react_register_bridge_call_resolver"),
        1,
        NativeFunction::from_copy_closure(
            move |_this: &JsValue, args: &[JsValue], ctx: &mut Context| {
                register_bridge_call_resolver_fn(args, ctx)
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

    // __react_call(name, args_json, call_id?) -> void
    context.register_global_callable(
        JsString::from("__react_call"),
        3,
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

fn register_bridge_call_resolver_fn(args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let callback = args.first().cloned().unwrap_or(JsValue::undefined());
    if !callback.is_callable() {
        return Err(JsError::from_opaque(JsValue::from(JsString::from(
            "__react_register_bridge_call_resolver expects a function",
        ))));
    }

    let global = ctx.global_object();
    global.set(
        JsString::from("__react_bridge_call_resolver"),
        callback,
        true,
        ctx,
    )?;
    log::debug!("Registered React bridge call resolver callback");
    Ok(JsValue::undefined())
}

fn flush_bridge_fn(bridge: &ReactBridge, ctx: &mut Context) -> JsResult<JsValue> {
    let updates = bridge.drain_state_updates();
    let call_results = bridge.drain_call_results();

    if !updates.is_empty() {
        let global = ctx.global_object();
        let dispatcher = global.get(JsString::from("__react_bridge_dispatcher"), ctx)?;
        let Some(callable) = dispatcher.as_callable() else {
            log::warn!(
                "__react_flush_bridge: no dispatcher registered ({} updates dropped)",
                updates.len()
            );
            // Still attempt to deliver call results below.
            deliver_call_results(&call_results, ctx)?;
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
    }

    deliver_call_results(&call_results, ctx)?;
    Ok(JsValue::undefined())
}

fn deliver_call_results(
    call_results: &[BridgeCallResult],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    if call_results.is_empty() {
        return Ok(JsValue::undefined());
    }

    let global = ctx.global_object();
    let resolver = global.get(JsString::from("__react_bridge_call_resolver"), ctx)?;
    let Some(callable) = resolver.as_callable() else {
        log::warn!(
            "__react_flush_bridge: no call resolver registered ({} results dropped)",
            call_results.len()
        );
        return Ok(JsValue::undefined());
    };

    for result in call_results {
        let payload = JsValue::from_json(&result.value, ctx)?;
        let call_args = [JsValue::from(result.id), payload];
        if let Err(err) = callable.call(&JsValue::undefined(), &call_args, ctx) {
            log::error!(
                "Failed to resolve bridge call {}: {err:?}",
                result.id
            );
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

    let call_id = args.get(2).and_then(|v| {
        if let Some(n) = v.as_number() {
            if n.is_finite() && n >= 0.0 {
                return Some(n as u64);
            }
        }
        v.as_string()
            .and_then(|s| s.to_std_string_escaped().parse::<u64>().ok())
    });

    let parsed: Value = serde_json::from_str(&args_json).unwrap_or(Value::Null);
    bridge.enqueue_call(name, parsed, call_id);
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
                json!({ "score": score.0 })
            });

        app.world_mut()
            .resource::<ReactBridge>()
            .enqueue_call("add".into(), json!(7), Some(42));

        process_react_bridge_calls(app.world_mut());
        assert_eq!(app.world().resource::<Score>().0, 7);
        assert!(!app.world().resource::<ReactBridge>().has_pending_calls());

        let results = app.world().resource::<ReactBridge>().drain_call_results();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, 42);
        assert_eq!(results[0].value, json!({ "score": 7 }));
    }

    #[test]
    fn missing_handler_still_enqueues_error_result() {
        let mut app = App::new();
        app.init_resource::<ReactBridge>();
        app.world_mut()
            .resource::<ReactBridge>()
            .enqueue_call("nope".into(), Value::Null, Some(1));

        process_react_bridge_calls(app.world_mut());
        let results = app.world().resource::<ReactBridge>().drain_call_results();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, 1);
        assert!(results[0].value.get("error").is_some());
    }

    #[test]
    fn resource_store_publishes_on_change() {
        let mut app = App::new();
        app.init_resource::<ReactBridge>();
        app.insert_resource(PlayerStats {
            hp: 100,
            max_hp: 100,
            score: 0,
        });
        app.add_systems(Update, sync_registered_resource_stores);
        app.world_mut()
            .resource::<ReactBridge>()
            .register_resource_store::<PlayerStats>("hud");

        // First frame publishes the initial resource snapshot.
        app.update();
        assert_eq!(
            app.world().resource::<ReactBridge>().get_state("hud"),
            Some(json!({ "hp": 100, "max_hp": 100, "score": 0 }))
        );
        let _ = app.world().resource::<ReactBridge>().drain_state_updates();

        // No change → no dirty republish.
        app.update();
        assert!(!app.world().resource::<ReactBridge>().has_pending_state());

        app.world_mut().resource_mut::<PlayerStats>().score = 10;
        app.update();
        let updates = app.world().resource::<ReactBridge>().drain_state_updates();
        assert_eq!(updates.len(), 1);
        assert_eq!(
            updates[0].1,
            json!({ "hp": 100, "max_hp": 100, "score": 10 })
        );
    }

    #[test]
    fn query_store_publishes_when_dirty() {
        let mut app = App::new();
        app.init_resource::<ReactBridge>();
        app.insert_resource(EnemyCount(2));
        app.add_systems(Update, sync_registered_query_stores);

        app.world_mut()
            .resource::<ReactBridge>()
            .register_query_store("enemies", |world| {
                let count = world
                    .get_resource::<EnemyCount>()
                    .map(|c| c.0)
                    .unwrap_or(0);
                json!({ "count": count })
            });

        // Registration marks dirty → first sync publishes.
        app.update();
        assert_eq!(
            app.world().resource::<ReactBridge>().get_state("enemies"),
            Some(json!({ "count": 2 }))
        );
        let _ = app.world().resource::<ReactBridge>().drain_state_updates();

        // No mark → no republish.
        app.update();
        assert!(!app.world().resource::<ReactBridge>().has_pending_state());

        // Change without mark → still no republish (dirty-driven).
        app.world_mut().resource_mut::<EnemyCount>().0 = 5;
        app.update();
        assert!(!app.world().resource::<ReactBridge>().has_pending_state());

        // mark_query_dirty → snapshot + publish.
        app.world_mut()
            .resource::<ReactBridge>()
            .mark_query_dirty("enemies");
        app.update();
        let updates = app.world().resource::<ReactBridge>().drain_state_updates();
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].1, json!({ "count": 5 }));

        // each_frame store skips publish when value unchanged.
        app.world_mut()
            .resource::<ReactBridge>()
            .register_query_store_each_frame("enemies_live", |world| {
                let count = world
                    .get_resource::<EnemyCount>()
                    .map(|c| c.0)
                    .unwrap_or(0);
                json!({ "count": count })
            });
        app.update();
        assert_eq!(
            app.world()
                .resource::<ReactBridge>()
                .get_state("enemies_live"),
            Some(json!({ "count": 5 }))
        );
        let _ = app.world().resource::<ReactBridge>().drain_state_updates();
        app.update();
        assert!(!app.world().resource::<ReactBridge>().has_pending_state());

        app.world_mut().resource_mut::<EnemyCount>().0 = 9;
        app.update();
        let live = app.world().resource::<ReactBridge>().drain_state_updates();
        assert_eq!(live.len(), 1);
        assert_eq!(live[0].0, "enemies_live");
        assert_eq!(live[0].1, json!({ "count": 9 }));
    }

    /// JSON shape contract for HUD `PlayerStats` (keys must match
    /// `examples/hud/ui/src/generated/` — regenerate via
    /// `./scripts/generate-bridge-types.sh`).
    #[test]
    fn player_stats_json_shape_matches_ts_contract() {
        let stats = PlayerStats {
            hp: 80,
            max_hp: 100,
            score: 1200,
        };
        let value = serde_json::to_value(&stats).expect("serialize");
        let obj = value.as_object().expect("object");
        let keys: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
        assert_eq!(keys, vec!["hp", "max_hp", "score"]);
        assert!(obj["hp"].as_i64().is_some());
        assert!(obj["max_hp"].as_i64().is_some());
        assert!(obj["score"].as_u64().is_some());
    }

    #[derive(Resource)]
    struct Score(i32);

    #[derive(Resource)]
    struct EnemyCount(u32);

    #[derive(Resource, Clone, Serialize)]
    struct PlayerStats {
        hp: i32,
        max_hp: i32,
        score: u32,
    }
}
