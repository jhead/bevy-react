//! React Native Functions
//!
//! Registers React reconciler native functions with the JS engine.

use boa_engine::{Context, JsError, JsResult, JsString, JsValue, NativeFunction};
use serde_json::Value;

use crate::{
    js::{JsEngineClient, JsEngineExtension},
    react::{
        bridge::{register_bridge_functions, ReactBridge},
        components_registry::ReactEntityMap,
        event_queue::ReactEventQueue, hmr::ReactReloadFlag, ReactClient,
        shim::register_environment_shims,
    },
};

/// React callback provider that registers native functions for the React reconciler.
#[derive(Clone)]
pub struct ReactJsExtension {
    client: ReactClient,
    event_queue: ReactEventQueue,
    bridge: ReactBridge,
    reload_flag: ReactReloadFlag,
    entity_map: ReactEntityMap,
}

impl ReactJsExtension {
    pub fn new(
        client: ReactClient,
        event_queue: ReactEventQueue,
        bridge: ReactBridge,
        reload_flag: ReactReloadFlag,
        entity_map: ReactEntityMap,
    ) -> Self {
        Self {
            client,
            event_queue,
            bridge,
            reload_flag,
            entity_map,
        }
    }
}

impl JsEngineExtension for ReactJsExtension {
    fn register(&self, context: &mut Context, _client: JsEngineClient) -> Result<(), JsError> {
        log::info!("Registering React native functions");
        register_environment_shims(context);
        register_react_functions(
            context,
            self.client.clone(),
            self.event_queue.clone(),
            self.reload_flag.clone(),
            self.entity_map.clone(),
        )?;
        register_bridge_functions(context, self.bridge.clone())?;
        Ok(())
    }
}

/// Register all React native functions in the JS global scope
fn register_react_functions(
    context: &mut Context,
    react_client: ReactClient,
    event_queue: ReactEventQueue,
    reload_flag: ReactReloadFlag,
    entity_map: ReactEntityMap,
) -> Result<(), JsError> {
    // __react_create_node(type: string, props_json: string) -> number
    context.register_global_callable(
        JsString::from("__react_create_node"),
        2,
        NativeFunction::from_copy_closure_with_captures(
            move |_this: &JsValue, args: &[JsValue], client: &ReactClient, ctx: &mut Context| {
                create_node_fn(args, client, ctx)
            },
            react_client.clone(),
        ),
    )?;

    // __react_create_text(content: string) -> number
    context.register_global_callable(
        JsString::from("__react_create_text"),
        1,
        NativeFunction::from_copy_closure_with_captures(
            move |_this: &JsValue, args: &[JsValue], client: &ReactClient, ctx: &mut Context| {
                create_text_fn(args, client, ctx)
            },
            react_client.clone(),
        ),
    )?;

    // __react_append_child(parent_id: number, child_id: number) -> void
    context.register_global_callable(
        JsString::from("__react_append_child"),
        2,
        NativeFunction::from_copy_closure_with_captures(
            move |_this: &JsValue, args: &[JsValue], client: &ReactClient, ctx: &mut Context| {
                append_child_fn(args, client, ctx)
            },
            react_client.clone(),
        ),
    )?;

    // __react_insert_before(parent_id: number, child_id: number, before_id: number) -> void
    context.register_global_callable(
        JsString::from("__react_insert_before"),
        3,
        NativeFunction::from_copy_closure_with_captures(
            move |_this: &JsValue, args: &[JsValue], client: &ReactClient, ctx: &mut Context| {
                insert_before_fn(args, client, ctx)
            },
            react_client.clone(),
        ),
    )?;

    // __react_remove_child(parent_id: number, child_id: number) -> void
    context.register_global_callable(
        JsString::from("__react_remove_child"),
        2,
        NativeFunction::from_copy_closure_with_captures(
            move |_this: &JsValue, args: &[JsValue], client: &ReactClient, ctx: &mut Context| {
                remove_child_fn(args, client, ctx)
            },
            react_client.clone(),
        ),
    )?;

    // __react_update_node(node_id: number, props_json: string) -> void
    context.register_global_callable(
        JsString::from("__react_update_node"),
        2,
        NativeFunction::from_copy_closure_with_captures(
            move |_this: &JsValue, args: &[JsValue], client: &ReactClient, ctx: &mut Context| {
                update_node_fn(args, client, ctx)
            },
            react_client.clone(),
        ),
    )?;

    // __react_update_text(node_id: number, content: string) -> void
    context.register_global_callable(
        JsString::from("__react_update_text"),
        2,
        NativeFunction::from_copy_closure_with_captures(
            move |_this: &JsValue, args: &[JsValue], client: &ReactClient, ctx: &mut Context| {
                update_text_fn(args, client, ctx)
            },
            react_client.clone(),
        ),
    )?;

    // __react_destroy_node(node_id: number) -> void
    context.register_global_callable(
        JsString::from("__react_destroy_node"),
        1,
        NativeFunction::from_copy_closure_with_captures(
            move |_this: &JsValue, args: &[JsValue], client: &ReactClient, ctx: &mut Context| {
                destroy_node_fn(args, client, ctx)
            },
            react_client.clone(),
        ),
    )?;

    // __react_clear_container() -> void
    context.register_global_callable(
        JsString::from("__react_clear_container"),
        0,
        NativeFunction::from_copy_closure_with_captures(
            move |_this: &JsValue, args: &[JsValue], client: &ReactClient, ctx: &mut Context| {
                clear_container_fn(args, client, ctx)
            },
            react_client.clone(),
        ),
    )?;

    // __react_register_event_dispatcher(callback) -> void
    // Stores the JS callback on the global object for structured host→JS events.
    context.register_global_callable(
        JsString::from("__react_register_event_dispatcher"),
        1,
        NativeFunction::from_copy_closure(
            move |_this: &JsValue, args: &[JsValue], ctx: &mut Context| {
                register_event_dispatcher_fn(args, ctx)
            },
        ),
    )?;

    // __react_flush_events() -> void
    // Drains the native event queue and invokes the registered dispatcher.
    context.register_global_callable(
        JsString::from("__react_flush_events"),
        0,
        NativeFunction::from_copy_closure_with_captures(
            move |_this: &JsValue,
                  _args: &[JsValue],
                  queue: &ReactEventQueue,
                  ctx: &mut Context| { flush_events_fn(queue, ctx) },
            event_queue.clone(),
        ),
    )?;

    // __react_request_reload() -> void
    // Vite HMR bridge: mark React roots dirty so Bevy re-executes the entry module.
    context.register_global_callable(
        JsString::from("__react_request_reload"),
        0,
        NativeFunction::from_copy_closure_with_captures(
            move |_this: &JsValue,
                  _args: &[JsValue],
                  flag: &ReactReloadFlag,
                  _ctx: &mut Context| {
                flag.request();
                Ok(JsValue::undefined())
            },
            reload_flag,
        ),
    )?;

    // __react_request_focus(nodeId, rootId?) -> void
    context.register_global_callable(
        JsString::from("__react_request_focus"),
        2,
        NativeFunction::from_copy_closure_with_captures(
            move |_this: &JsValue,
                  args: &[JsValue],
                  queue: &ReactEventQueue,
                  _ctx: &mut Context| { request_focus_fn(args, queue) },
            event_queue.clone(),
        ),
    )?;

    // __react_request_blur() -> void
    context.register_global_callable(
        JsString::from("__react_request_blur"),
        0,
        NativeFunction::from_copy_closure_with_captures(
            move |_this: &JsValue,
                  _args: &[JsValue],
                  queue: &ReactEventQueue,
                  _ctx: &mut Context| {
                queue.request_blur();
                Ok(JsValue::undefined())
            },
            event_queue,
        ),
    )?;

    // __react_entity_id(nodeId) -> number | null
    // Returns Bevy Entity::to_bits() for a React node once the ECS entity exists.
    context.register_global_callable(
        JsString::from("__react_entity_id"),
        1,
        NativeFunction::from_copy_closure_with_captures(
            move |_this: &JsValue,
                  args: &[JsValue],
                  map: &ReactEntityMap,
                  _ctx: &mut Context| { entity_id_fn(args, map) },
            entity_map,
        ),
    )?;

    #[cfg(feature = "binary_ops")]
    {
        // __react_commit_ops(bytes: Uint8Array | ArrayBuffer) -> void
        // Dual path: one BRRP frame → many ReactClientProto messages.
        context.register_global_callable(
            JsString::from("__react_commit_ops"),
            1,
            NativeFunction::from_copy_closure_with_captures(
                move |_this: &JsValue, args: &[JsValue], client: &ReactClient, ctx: &mut Context| {
                    commit_ops_fn(args, client, ctx)
                },
                react_client.clone(),
            ),
        )?;
    }

    log::debug!("Registered React native functions");
    Ok(())
}

/// __react_entity_id(nodeId) -> Entity::to_bits() or null
fn entity_id_fn(args: &[JsValue], map: &ReactEntityMap) -> JsResult<JsValue> {
    let node_id = args
        .first()
        .and_then(|v| v.as_number())
        .map(|n| n as u64)
        .ok_or_else(|| {
            JsError::from_opaque(JsValue::from(JsString::from(
                "__react_entity_id expects a node id number",
            )))
        })?;

    match map.get(node_id) {
        Some(bits) => Ok(JsValue::from(bits as f64)),
        None => Ok(JsValue::null()),
    }
}

/// __react_request_focus(nodeId, rootId?)
fn request_focus_fn(args: &[JsValue], queue: &ReactEventQueue) -> JsResult<JsValue> {
    let node_id = args
        .first()
        .and_then(|v| v.as_number())
        .map(|n| n as u64)
        .ok_or_else(|| {
            JsError::from_opaque(JsValue::from(JsString::from(
                "__react_request_focus expects a node id number",
            )))
        })?;

    let root_id = args
        .get(1)
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped());

    queue.request_focus(node_id, root_id);
    Ok(JsValue::undefined())
}

/// __react_register_event_dispatcher(callback)
fn register_event_dispatcher_fn(args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let callback = args.first().cloned().unwrap_or(JsValue::undefined());
    if !callback.is_callable() {
        return Err(JsError::from_opaque(JsValue::from(JsString::from(
            "__react_register_event_dispatcher expects a function",
        ))));
    }

    let global = ctx.global_object();
    global.set(
        JsString::from("__react_event_dispatcher"),
        callback,
        true,
        ctx,
    )?;
    log::debug!("Registered React event dispatcher callback");
    Ok(JsValue::undefined())
}

/// __react_flush_events()
fn flush_events_fn(queue: &ReactEventQueue, ctx: &mut Context) -> JsResult<JsValue> {
    let events = queue.drain();
    if events.is_empty() {
        return Ok(JsValue::undefined());
    }

    let global = ctx.global_object();
    let dispatcher = global.get(JsString::from("__react_event_dispatcher"), ctx)?;
    let Some(callable) = dispatcher.as_callable() else {
        log::warn!(
            "__react_flush_events: no dispatcher registered ({} events dropped)",
            events.len()
        );
        return Ok(JsValue::undefined());
    };

    for event in events {
        let payload_json: Value =
            serde_json::from_str(&event.payload_json).unwrap_or(Value::Null);
        let payload = JsValue::from_json(&payload_json, ctx)?;

        let call_args = [
            JsValue::from(JsString::from(event.root_id.as_str())),
            JsValue::from(event.node_id as f64),
            JsValue::from(JsString::from(event.event_type.as_str())),
            payload,
        ];

        if let Err(err) = callable.call(&JsValue::undefined(), &call_args, ctx) {
            log::error!(
                "Failed to dispatch React event {} on node {}: {:?}",
                event.event_type,
                event.node_id,
                err
            );
        }
    }

    Ok(JsValue::undefined())
}

/// __react_create_node(type, props_json) -> node_id
fn create_node_fn(args: &[JsValue], client: &ReactClient, _ctx: &mut Context) -> JsResult<JsValue> {
    let root_id = args
        .first()
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_else(|| "root".to_string());

    let node_type = args
        .get(1)
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_else(|| "node".to_string());

    let props_json = args
        .get(2)
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_else(|| "{}".to_string());

    let node_id = client.create_node(root_id, node_type, props_json);
    Ok(JsValue::from(node_id as f64))
}

/// __react_create_text(content) -> node_id
fn create_text_fn(args: &[JsValue], client: &ReactClient, _ctx: &mut Context) -> JsResult<JsValue> {
    let root_id = args
        .first()
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_else(|| "root".to_string());

    let content = args
        .get(1)
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();

    let node_id = client.create_text(root_id, content);
    Ok(JsValue::from(node_id as f64))
}

/// __react_append_child(parent_id, child_id)
fn append_child_fn(
    args: &[JsValue],
    client: &ReactClient,
    _ctx: &mut Context,
) -> JsResult<JsValue> {
    let root_id = args
        .first()
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_else(|| "root".to_string());

    let parent_id = args
        .get(1)
        .and_then(|v| v.as_number())
        .map(|n| n as u64)
        .unwrap_or(0);

    let child_id = args
        .get(2)
        .and_then(|v| v.as_number())
        .map(|n| n as u64)
        .unwrap_or(0);

    client.append_child(root_id, parent_id, child_id);
    Ok(JsValue::undefined())
}

/// __react_insert_before(parent_id, child_id, before_id)
fn insert_before_fn(
    args: &[JsValue],
    client: &ReactClient,
    _ctx: &mut Context,
) -> JsResult<JsValue> {
    let root_id = args
        .first()
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_else(|| "root".to_string());

    let parent_id = args
        .get(1)
        .and_then(|v| v.as_number())
        .map(|n| n as u64)
        .unwrap_or(0);

    let child_id = args
        .get(2)
        .and_then(|v| v.as_number())
        .map(|n| n as u64)
        .unwrap_or(0);

    let before_id = args
        .get(3)
        .and_then(|v| v.as_number())
        .map(|n| n as u64)
        .unwrap_or(0);

    client.insert_before(root_id, parent_id, child_id, before_id);
    Ok(JsValue::undefined())
}

/// __react_remove_child(parent_id, child_id)
fn remove_child_fn(
    args: &[JsValue],
    client: &ReactClient,
    _ctx: &mut Context,
) -> JsResult<JsValue> {
    let root_id = args
        .first()
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_else(|| "root".to_string());

    let parent_id = args
        .get(1)
        .and_then(|v| v.as_number())
        .map(|n| n as u64)
        .unwrap_or(0);

    let child_id = args
        .get(2)
        .and_then(|v| v.as_number())
        .map(|n| n as u64)
        .unwrap_or(0);

    client.remove_child(root_id, parent_id, child_id);
    Ok(JsValue::undefined())
}

/// __react_update_node(node_id, props_json)
fn update_node_fn(args: &[JsValue], client: &ReactClient, _ctx: &mut Context) -> JsResult<JsValue> {
    let root_id = args
        .first()
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_else(|| "root".to_string());

    let node_id = args
        .get(1)
        .and_then(|v| v.as_number())
        .map(|n| n as u64)
        .unwrap_or(0);

    let props_json = args
        .get(2)
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_else(|| "{}".to_string());

    client.update_node(root_id, node_id, props_json);
    Ok(JsValue::undefined())
}

/// __react_update_text(node_id, content)
fn update_text_fn(args: &[JsValue], client: &ReactClient, _ctx: &mut Context) -> JsResult<JsValue> {
    let root_id = args
        .first()
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_else(|| "root".to_string());

    let node_id = args
        .get(1)
        .and_then(|v| v.as_number())
        .map(|n| n as u64)
        .unwrap_or(0);

    let content = args
        .get(2)
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();

    client.update_text(root_id, node_id, content);
    Ok(JsValue::undefined())
}

/// __react_destroy_node(node_id)
fn destroy_node_fn(
    args: &[JsValue],
    client: &ReactClient,
    _ctx: &mut Context,
) -> JsResult<JsValue> {
    let root_id = args
        .first()
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_else(|| "root".to_string());

    let node_id = args
        .get(1)
        .and_then(|v| v.as_number())
        .map(|n| n as u64)
        .unwrap_or(0);

    client.destroy_node(root_id, node_id);
    Ok(JsValue::undefined())
}

/// __react_clear_container()
fn clear_container_fn(
    args: &[JsValue],
    client: &ReactClient,
    _ctx: &mut Context,
) -> JsResult<JsValue> {
    let root_id = args
        .first()
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_else(|| "root".to_string());

    client.clear_container(root_id);
    Ok(JsValue::undefined())
}

/// __react_commit_ops(bytes: Uint8Array | ArrayBuffer)
#[cfg(feature = "binary_ops")]
fn commit_ops_fn(
    args: &[JsValue],
    client: &ReactClient,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    use boa_engine::object::builtins::{JsArrayBuffer, JsTypedArray};

    let obj = args
        .first()
        .and_then(|v| v.as_object())
        .ok_or_else(|| {
            JsError::from_opaque(JsValue::from(JsString::from(
                "__react_commit_ops expects a Uint8Array or ArrayBuffer",
            )))
        })?;

    let bytes = if let Ok(buf) = JsArrayBuffer::from_object(obj.clone()) {
        buf.data()
            .map(|d| d.to_vec())
            .ok_or_else(|| {
                JsError::from_opaque(JsValue::from(JsString::from(
                    "__react_commit_ops: ArrayBuffer is detached",
                )))
            })?
    } else {
        let typed = JsTypedArray::from_object(obj).map_err(|_| {
            JsError::from_opaque(JsValue::from(JsString::from(
                "__react_commit_ops expects a Uint8Array or ArrayBuffer",
            )))
        })?;
        let byte_length = typed.byte_length(ctx)?;
        let byte_offset = typed.byte_offset(ctx)?;
        let buffer_val = typed.buffer(ctx)?;
        let buffer_obj = buffer_val.as_object().ok_or_else(|| {
            JsError::from_opaque(JsValue::from(JsString::from(
                "__react_commit_ops: TypedArray buffer missing",
            )))
        })?;
        let buf = JsArrayBuffer::from_object(buffer_obj.clone()).map_err(|_| {
            JsError::from_opaque(JsValue::from(JsString::from(
                "__react_commit_ops: TypedArray buffer is not an ArrayBuffer",
            )))
        })?;
        let data = buf.data().ok_or_else(|| {
            JsError::from_opaque(JsValue::from(JsString::from(
                "__react_commit_ops: ArrayBuffer is detached",
            )))
        })?;
        let end = byte_offset
            .checked_add(byte_length)
            .filter(|e| *e <= data.len())
            .ok_or_else(|| {
                JsError::from_opaque(JsValue::from(JsString::from(
                    "__react_commit_ops: TypedArray view out of bounds",
                )))
            })?;
        data[byte_offset..end].to_vec()
    };

    client.commit_binary_ops(&bytes).map_err(|e| {
        JsError::from_opaque(JsValue::from(JsString::from(format!(
            "__react_commit_ops decode failed: {e:?}"
        ))))
    })?;

    Ok(JsValue::undefined())
}
