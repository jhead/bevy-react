use bevy::prelude::*;

use crate::react::client::ReactClientProto;
use crate::react::style::{json_to_style, parse_color, parse_props, parse_val};
use crate::react::systems::types::*;

/// Process incoming React messages and apply them to the ECS
pub fn process_react_messages(
    mut commands: Commands,
    receiver: Option<Res<ReactMessageReceiver>>,
    asset_server: Res<AssetServer>,
    root_map: Res<ReactRootMap>,
    mut contexts: Query<(Entity, Mut<ReactContext>)>,
) {
    let Some(receiver) = receiver else {
        return;
    };

    // Process all pending messages
    while let Some(message) = receiver.0.try_recv() {
        log::info!("Processing React message: {:?}", message);
        match message {
            ReactClientProto::CreateNode {
                root_id,
                node_id,
                node_type,
                props_json,
            } => {
                let Some(root) = root_map.roots.get(&root_id) else {
                    continue;
                };
                let Ok((_, mut context)) = contexts.get_mut(*root) else {
                    continue;
                };

                handle_create_node(
                    &mut commands,
                    context.as_mut(),
                    &asset_server,
                    node_id,
                    &node_type,
                    &props_json,
                );
            }

            ReactClientProto::CreateText {
                root_id,
                node_id,
                content,
            } => {
                let Some(root) = root_map.roots.get(&root_id) else {
                    continue;
                };
                let Ok((_, mut context)) = contexts.get_mut(*root) else {
                    continue;
                };

                handle_create_text(&mut commands, context.as_mut(), node_id, &content);
            }

            ReactClientProto::AppendChild {
                root_id,
                parent_id,
                child_id,
            } => {
                log::info!(
                    "Appending child: root_id={}, parent_id={}, child_id={}",
                    root_id,
                    parent_id,
                    child_id
                );
                let Some(root) = root_map.roots.get(&root_id) else {
                    continue;
                };
                let Ok((_, mut context)) = contexts.get_mut(*root) else {
                    continue;
                };

                handle_append_child(&mut commands, context.as_mut(), parent_id, child_id);
            }

            ReactClientProto::RemoveChild {
                root_id,
                parent_id,
                child_id,
            } => {
                let Some(root) = root_map.roots.get(&root_id) else {
                    continue;
                };
                let Ok((_, mut context)) = contexts.get_mut(*root) else {
                    continue;
                };

                handle_remove_child(&mut commands, context.as_mut(), parent_id, child_id);
            }

            ReactClientProto::UpdateNode {
                root_id,
                node_id,
                props_json,
            } => {
                let Some(root) = root_map.roots.get(&root_id) else {
                    continue;
                };
                let Ok((_, mut context)) = contexts.get_mut(*root) else {
                    continue;
                };

                handle_update_node(
                    &mut commands,
                    context.as_mut(),
                    &asset_server,
                    node_id,
                    &props_json,
                );
            }

            ReactClientProto::UpdateText {
                root_id,
                node_id,
                content,
            } => {
                let Some(root) = root_map.roots.get(&root_id) else {
                    continue;
                };
                let Ok((_, mut context)) = contexts.get_mut(*root) else {
                    continue;
                };

                handle_update_text(&mut commands, context.as_mut(), node_id, &content);
            }

            ReactClientProto::DestroyNode { root_id, node_id } => {
                let Some(root) = root_map.roots.get(&root_id) else {
                    continue;
                };
                let Ok((_, mut context)) = contexts.get_mut(*root) else {
                    continue;
                };

                handle_destroy_node(&mut commands, &mut context, node_id);
            }

            ReactClientProto::ClearContainer { root_id } => {
                let Some(root) = root_map.roots.get(&root_id) else {
                    log::error!("Failed to get root for clear container: {}", root_id);
                    continue;
                };
                let Ok((_, mut context)) = contexts.get_mut(*root) else {
                    log::error!("Failed to get context for root: {}", root_id);
                    continue;
                };

                log::info!("Clearing container: {}", root_id);
                handle_clear_container(&mut commands, &mut context);
            }

            ReactClientProto::Complete => {
                log::info!("React batch complete");
            }
        }
    }
}

/// Create a new UI node
fn handle_create_node(
    commands: &mut Commands,
    context: &mut ReactContext,
    asset_server: &AssetServer,
    node_id: u64,
    node_type: &str,
    props_json: &str,
) {
    let props = parse_props(props_json);
    let style = props.style.as_ref().map(json_to_style).unwrap_or_default();

    let mut entity_commands = match node_type {
        "bevy-button" => {
            // Button node
            let cmd = commands.spawn((Button, style, ReactNode { node_id }));
            cmd
        }

        "bevy-image" => {
            // Image node
            let mut cmd = commands.spawn((style, ReactNode { node_id }));

            if let Some(ref image_path) = props.image {
                let image_handle: Handle<Image> = asset_server.load(image_path);
                cmd.insert(ImageNode::new(image_handle));
            }
            cmd
        }

        "bevy-text" => {
            // Text node with content
            let content = props.content.as_deref().unwrap_or("");
            let mut cmd = commands.spawn((
                Text::new(content.to_string()),
                ReactNode { node_id },
                ReactTextNode,
            ));

            // Apply text styling
            if let Some(ref style_props) = props.style {
                // Apply text color via TextColor component
                if let Some(ref color_str) = style_props.color {
                    if let Some(color) = parse_color(color_str) {
                        cmd.insert(TextColor(color));
                    }
                }
                // Apply font size via TextFont component
                if let Some(ref font_size) = style_props.font_size {
                    let size = parse_val(&font_size.0);
                    if let Val::Px(px) = size {
                        cmd.insert(TextFont::from_font_size(px));
                    }
                }
            }

            cmd
        }

        "bevy-node" => {
            // Regular node
            commands.spawn((style, ReactNode { node_id }))
        }

        "bevy-text-input" => {
            // Text input container node - marked as focusable for keyboard events
            commands.spawn((style, ReactNode { node_id }, Focusable))
        }

        _ => {
            log::warn!("Unknown node type: {}", node_type);
            return;
        }
    };

    // Apply interaction component if needed
    entity_commands.insert(Interaction::default());

    // Apply colors (common for non-text nodes)
    // TODO: why is this here?
    if node_type != "bevy-text" {
        if let Some(ref style_props) = props.style {
            if let Some(ref bg) = style_props.background_color {
                if let Some(color) = parse_color(bg) {
                    entity_commands.insert(BackgroundColor(color));
                }
            }
            if let Some(ref bc) = style_props.border_color {
                if let Some(color) = parse_color(bc) {
                    entity_commands.insert(BorderColor::all(color));
                }
            }
        }
    }

    context.nodes.insert(node_id, entity_commands.id());
    let entity = entity_commands.id(); // capture id for logging

    log::info!(
        "Created {} node: id={} entity={:?}",
        node_type,
        node_id,
        entity
    );
}

/// Create a text node
fn handle_create_text(
    commands: &mut Commands,
    context: &mut ReactContext,
    node_id: u64,
    content: &str,
) {
    let entity = commands
        .spawn((
            Text::new(content.to_string()),
            ReactNode { node_id },
            ReactTextNode,
        ))
        .id();

    context.nodes.insert(node_id, entity);
    log::info!("Created text node: id={} entity={:?} text={}", node_id, entity, content);
}

/// Append a child to a parent
fn handle_append_child(
    commands: &mut Commands,
    context: &ReactContext,
    parent_id: u64,
    child_id: u64,
) {
    // Special case: parent_id 0 means root container
    let parent_entity = if parent_id == 0 {
        context.root
    } else {
        context.nodes.get(&parent_id).copied()
    };

    let child_entity = context.nodes.get(&child_id).copied();

    match (parent_entity, child_entity) {
        (Some(parent), Some(child)) => {
            commands.entity(parent).add_child(child);
            log::info!(
                "Appended child: parent={:?} child={:?}",
                parent_id,
                child_id
            );
        }
        _ => {
            log::warn!(
                "Failed to append child: parent_id={} child_id={} (entities not found)",
                parent_id,
                child_id
            );
        }
    }
}

/// Remove a child from a parent
fn handle_remove_child(
    commands: &mut Commands,
    context: &ReactContext,
    parent_id: u64,
    child_id: u64,
) {
    let child_entity = context.nodes.get(&child_id).copied();

    if let Some(child) = child_entity {
        commands.entity(child).remove_parent_in_place();
        log::info!("Removed child: parent={} child={:?}", parent_id, child_id);
    } else {
        log::warn!(
            "Failed to remove child: child_id={} (entity not found)",
            child_id
        );
    }
}

/// Update node properties
fn handle_update_node(
    commands: &mut Commands,
    context: &ReactContext,
    asset_server: &AssetServer,
    node_id: u64,
    props_json: &str,
) {
    let Some(entity) = context.nodes.get(&node_id).copied() else {
        log::warn!("Failed to update node: id={} (entity not found)", node_id);
        return;
    };

    let props = parse_props(props_json);

    if let Some(ref style_props) = props.style {
        let style = json_to_style(style_props);
        commands.entity(entity).insert(style);

        // Update colors
        if let Some(ref bg) = style_props.background_color {
            if let Some(color) = parse_color(bg) {
                commands.entity(entity).insert(BackgroundColor(color));
            }
        }
        if let Some(ref bc) = style_props.border_color {
            if let Some(color) = parse_color(bc) {
                commands.entity(entity).insert(BorderColor::all(color));
            }
        }
    }

    // Update image if provided
    if let Some(ref image_path) = props.image {
        let image_handle: Handle<Image> = asset_server.load(image_path);
        commands.entity(entity).insert(ImageNode::new(image_handle));
    }

    // Update text content if provided (for bevy-text nodes)
    if let Some(ref content) = props.content {
        commands.entity(entity).insert(Text::new(content.clone()));
        log::info!("Updated text content: id={} content={}", node_id, content);
    }

    log::info!("Updated node: id={}", node_id);
}

/// Update text content
fn handle_update_text(
    commands: &mut Commands,
    context: &ReactContext,
    node_id: u64,
    content: &str,
) {
    let Some(entity) = context.nodes.get(&node_id).copied() else {
        log::warn!("Failed to update text: id={} (entity not found)", node_id);
        return;
    };

    commands
        .entity(entity)
        .insert(Text::new(content.to_string()));
    log::info!("Updated text: id={} content={}", node_id, content);
}

/// Destroy a node
fn handle_destroy_node(commands: &mut Commands, context: &mut ReactContext, node_id: u64) {
    if let Some(entity) = context.nodes.remove(&node_id) {
        commands.entity(entity).despawn();
        log::info!("Destroyed node: id={} entity={:?}", node_id, entity);
    } else {
        log::warn!("Failed to destroy node: id={} (not found)", node_id);
    }
}

/// Clear all children from the root container
fn handle_clear_container(_commands: &mut Commands, _context: &mut ReactContext) {
    log::info!("React clear_container called (no-op for now)");
}
