//! React Native Functions
//!
//! Registers React reconciler native functions with the JS engine.

use boa_engine::{Context, JsError, JsResult, JsString, JsValue, NativeFunction};

use crate::{
    js::{JsEngineClient, JsEngineExtension},
    react::{ReactClient, shim::register_environment_shims},
};

/// React callback provider that registers native functions for the React reconciler.
#[derive(Clone)]
pub struct ReactJsExtension {
    client: ReactClient,
}

impl ReactJsExtension {
    pub fn new(client: ReactClient) -> Self {
        Self { client }
    }
}

impl JsEngineExtension for ReactJsExtension {
    fn register(&self, context: &mut Context, _client: JsEngineClient) -> Result<(), JsError> {
        log::info!("Registering React native functions");
        register_environment_shims(context);
        register_react_functions(context, self.client.clone());
        Ok(())
    }
}

/// Register all React native functions in the JS global scope
fn register_react_functions(context: &mut Context, react_client: ReactClient) {
    // __react_create_node(type: string, props_json: string) -> number
    context
        .register_global_callable(
            JsString::from("__react_create_node"),
            2,
            NativeFunction::from_copy_closure_with_captures(
                move |_this: &JsValue,
                      args: &[JsValue],
                      client: &ReactClient,
                      ctx: &mut Context| { create_node_fn(args, client, ctx) },
                react_client.clone(),
            ),
        )
        .expect("Failed to register __react_create_node");

    // __react_create_text(content: string) -> number
    context
        .register_global_callable(
            JsString::from("__react_create_text"),
            1,
            NativeFunction::from_copy_closure_with_captures(
                move |_this: &JsValue,
                      args: &[JsValue],
                      client: &ReactClient,
                      ctx: &mut Context| { create_text_fn(args, client, ctx) },
                react_client.clone(),
            ),
        )
        .expect("Failed to register __react_create_text");

    // __react_append_child(parent_id: number, child_id: number) -> void
    context
        .register_global_callable(
            JsString::from("__react_append_child"),
            2,
            NativeFunction::from_copy_closure_with_captures(
                move |_this: &JsValue,
                      args: &[JsValue],
                      client: &ReactClient,
                      ctx: &mut Context| { append_child_fn(args, client, ctx) },
                react_client.clone(),
            ),
        )
        .expect("Failed to register __react_append_child");

    // __react_remove_child(parent_id: number, child_id: number) -> void
    context
        .register_global_callable(
            JsString::from("__react_remove_child"),
            2,
            NativeFunction::from_copy_closure_with_captures(
                move |_this: &JsValue,
                      args: &[JsValue],
                      client: &ReactClient,
                      ctx: &mut Context| { remove_child_fn(args, client, ctx) },
                react_client.clone(),
            ),
        )
        .expect("Failed to register __react_remove_child");

    // __react_update_node(node_id: number, props_json: string) -> void
    context
        .register_global_callable(
            JsString::from("__react_update_node"),
            2,
            NativeFunction::from_copy_closure_with_captures(
                move |_this: &JsValue,
                      args: &[JsValue],
                      client: &ReactClient,
                      ctx: &mut Context| { update_node_fn(args, client, ctx) },
                react_client.clone(),
            ),
        )
        .expect("Failed to register __react_update_node");

    // __react_update_text(node_id: number, content: string) -> void
    context
        .register_global_callable(
            JsString::from("__react_update_text"),
            2,
            NativeFunction::from_copy_closure_with_captures(
                move |_this: &JsValue,
                      args: &[JsValue],
                      client: &ReactClient,
                      ctx: &mut Context| { update_text_fn(args, client, ctx) },
                react_client.clone(),
            ),
        )
        .expect("Failed to register __react_update_text");

    // __react_destroy_node(node_id: number) -> void
    context
        .register_global_callable(
            JsString::from("__react_destroy_node"),
            1,
            NativeFunction::from_copy_closure_with_captures(
                move |_this: &JsValue,
                      args: &[JsValue],
                      client: &ReactClient,
                      ctx: &mut Context| { destroy_node_fn(args, client, ctx) },
                react_client.clone(),
            ),
        )
        .expect("Failed to register __react_destroy_node");

    // __react_clear_container() -> void
    context
        .register_global_callable(
            JsString::from("__react_clear_container"),
            0,
            NativeFunction::from_copy_closure_with_captures(
                move |_this: &JsValue,
                      args: &[JsValue],
                      client: &ReactClient,
                      ctx: &mut Context| { clear_container_fn(args, client, ctx) },
                react_client.clone(),
            ),
        )
        .expect("Failed to register __react_clear_container");

    log::debug!("Registered React native functions");
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
