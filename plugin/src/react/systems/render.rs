use bevy::prelude::*;
use bevy::text::TextLayout;

use crate::react::client::ReactClientProto;
use crate::react::style::{
    json_to_style, parse_color, parse_props, parse_val, style_font_family, style_line_height,
    style_object_fit, style_opacity, style_text_align, style_tint, style_to_background_gradient,
    style_to_border_color, style_to_border_radius, style_to_box_shadow, StyleProps,
};
use crate::react::systems::types::*;

/// Process incoming React messages and apply them to the ECS
pub fn process_react_messages(
    mut commands: Commands,
    receiver: Option<Res<ReactMessageReceiver>>,
    asset_server: Res<AssetServer>,
    root_map: Res<ReactRootMap>,
    mut contexts: Query<(Entity, Mut<ReactContext>)>,
    text_nodes: Query<(), With<ReactTextNode>>,
) {
    let Some(receiver) = receiver else {
        return;
    };

    // Process all pending messages
    while let Some(message) = receiver.0.try_recv() {
        log::trace!("Processing React message: {:?}", message);
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
                log::debug!(
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

                let is_text = context
                    .nodes
                    .get(&node_id)
                    .map(|entity| text_nodes.get(*entity).is_ok())
                    .unwrap_or(false);

                handle_update_node(
                    &mut commands,
                    context.as_mut(),
                    &asset_server,
                    node_id,
                    &props_json,
                    is_text,
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

            ReactClientProto::InsertBefore {
                root_id,
                parent_id,
                child_id,
                before_id,
            } => {
                let Some(root) = root_map.roots.get(&root_id) else {
                    continue;
                };
                let Ok((_, mut context)) = contexts.get_mut(*root) else {
                    continue;
                };

                handle_insert_before(
                    &mut commands,
                    context.as_mut(),
                    parent_id,
                    child_id,
                    before_id,
                );
            }

            ReactClientProto::DestroyNode { root_id, node_id } => {
                let Some(root) = root_map.roots.get(&root_id) else {
                    continue;
                };
                let Ok((context_entity, mut context)) = contexts.get_mut(*root) else {
                    continue;
                };

                handle_destroy_node(&mut commands, context_entity, context.as_mut(), node_id);
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

                log::debug!("Clearing container: {}", root_id);
                handle_clear_container(&mut commands, *root, &mut context);
            }

            ReactClientProto::Complete => {
                log::trace!("React batch complete");
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
            commands.spawn((Button, style, ReactNode { node_id }))
        }

        "bevy-image" => {
            // Image node
            let mut cmd = commands.spawn((style, ReactNode { node_id }));

            if let Some(image_path) = props.src.clone().or(props.image.clone()) {
                let image_handle: Handle<Image> = asset_server.load(image_path);
                let mut image_node = ImageNode::new(image_handle);
                if let Some(ref style_props) = props.style {
                    if let Some(mode) = style_object_fit(style_props) {
                        image_node.image_mode = mode;
                    }
                    if let Some(tint) = style_tint(style_props) {
                        image_node.color = tint;
                    }
                    if let Some(opacity) = style_opacity(style_props) {
                        image_node.color.set_alpha(opacity);
                    }
                }
                cmd.insert(image_node);
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
                apply_text_style(&mut cmd, style_props, asset_server);
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

    // Apply visual components (common for non-text nodes)
    if node_type != "bevy-text"
        && let Some(ref style_props) = props.style
    {
        apply_visual_style(&mut entity_commands, style_props);
    }

    // Apply ZIndex if specified
    if let Some(ref style_props) = props.style
        && let Some(z) = style_props.z_index
    {
        entity_commands.insert(ZIndex(z));
    }

    context.nodes.insert(node_id, entity_commands.id());
    let entity = entity_commands.id(); // capture id for logging

    log::debug!(
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
    log::debug!("Created text node: id={} entity={:?} text={}", node_id, entity, content);
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
            log::debug!(
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
        log::debug!("Removed child: parent={} child={:?}", parent_id, child_id);
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
    is_text: bool,
) {
    let Some(entity) = context.nodes.get(&node_id).copied() else {
        log::warn!("Failed to update node: id={} (entity not found)", node_id);
        return;
    };

    let props = parse_props(props_json);

    if is_text {
        // Text nodes: only Text / TextColor / TextFont / layout — never BackgroundColor
        if let Some(ref content) = props.content {
            commands.entity(entity).insert(Text::new(content.clone()));
        }
        match props.style.as_ref() {
            Some(style_props) => {
                apply_text_style_commands(commands, entity, style_props, asset_server);
            }
            None => {
                commands.entity(entity).remove::<TextColor>();
                commands.entity(entity).remove::<TextFont>();
                commands.entity(entity).remove::<TextLayout>();
            }
        }
        log::debug!("Updated text node: id={}", node_id);
        return;
    }

    match props.style.as_ref() {
        Some(style_props) => {
            let style = json_to_style(style_props);
            commands.entity(entity).insert(style);
            apply_visual_style_commands(commands, entity, style_props);

            // ZIndex
            match style_props.z_index {
                Some(z) => {
                    commands.entity(entity).insert(ZIndex(z));
                }
                None => {
                    commands.entity(entity).remove::<ZIndex>();
                }
            }
        }
        None => {
            // Entire style prop cleared — drop optional visual overrides and reset layout
            commands.entity(entity).insert(Node::default());
            commands.entity(entity).remove::<BackgroundColor>();
            commands.entity(entity).remove::<BorderColor>();
            commands.entity(entity).remove::<BorderRadius>();
            commands.entity(entity).remove::<BoxShadow>();
            commands.entity(entity).remove::<BackgroundGradient>();
            commands.entity(entity).remove::<ZIndex>();
            commands.entity(entity).remove::<Visibility>();
        }
    }

    // Update image if provided
    if let Some(image_path) = props.src.clone().or(props.image.clone()) {
        let image_handle: Handle<Image> = asset_server.load(image_path);
        let mut image_node = ImageNode::new(image_handle);
        if let Some(ref style_props) = props.style {
            if let Some(mode) = style_object_fit(style_props) {
                image_node.image_mode = mode;
            }
            if let Some(tint) = style_tint(style_props) {
                image_node.color = tint;
            }
            if let Some(opacity) = style_opacity(style_props) {
                image_node.color.set_alpha(opacity);
            }
        }
        commands.entity(entity).insert(image_node);
    }

    log::debug!("Updated node: id={}", node_id);
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
    log::debug!("Updated text: id={} content={}", node_id, content);
}

/// Destroy a node and its descendants, purging ReactContext maps
fn handle_destroy_node(
    commands: &mut Commands,
    context_entity: Entity,
    context: &mut ReactContext,
    node_id: u64,
) {
    let Some(entity) = context.nodes.remove(&node_id) else {
        // Already destroyed (e.g. removeChild + detachDeletedInstance)
        return;
    };

    // Purge descendant node_id mappings and recursively despawn the entity tree.
    // Children relationships are only readable once commands are applied, so defer.
    commands.queue(move |world: &mut World| {
        let mut subtree = Vec::new();
        collect_entity_subtree(world, entity, &mut subtree);

        if let Some(mut ctx) = world.get_mut::<ReactContext>(context_entity) {
            ctx.nodes.retain(|_, mapped| !subtree.contains(mapped));
        }

        if world.get_entity(entity).is_ok() {
            world.entity_mut(entity).despawn();
        }

        log::debug!(
            "Destroyed node: id={} entity={:?} (subtree size={})",
            node_id,
            entity,
            subtree.len()
        );
    });
}

fn collect_entity_subtree(world: &World, entity: Entity, out: &mut Vec<Entity>) {
    out.push(entity);
    if let Some(children) = world.get::<Children>(entity) {
        for child in children.iter() {
            collect_entity_subtree(world, child, out);
        }
    }
}

/// Insert a child before another child in a parent
fn handle_insert_before(
    commands: &mut Commands,
    context: &ReactContext,
    parent_id: u64,
    child_id: u64,
    before_id: u64,
) {
    let parent_entity = if parent_id == 0 {
        context.root
    } else {
        context.nodes.get(&parent_id).copied()
    };

    let child_entity = context.nodes.get(&child_id).copied();
    let before_entity = context.nodes.get(&before_id).copied();

    match (parent_entity, child_entity, before_entity) {
        (Some(parent), Some(child), Some(before)) => {
            // First remove child from current parent if it has one
            commands.entity(child).remove_parent_in_place();
            // Find the index of before_entity among parent's children and insert there
            commands.queue(move |world: &mut World| {
                let children: Vec<Entity> = world
                    .entity(parent)
                    .get::<Children>()
                    .map(|c| c.iter().collect())
                    .unwrap_or_default();

                let index = children
                    .iter()
                    .position(|&e| e == before)
                    .unwrap_or(children.len());

                world.entity_mut(parent).insert_children(index, &[child]);
            });
            log::debug!(
                "Inserted child {} before {} in parent {}",
                child_id,
                before_id,
                parent_id
            );
        }
        _ => {
            log::warn!(
                "Failed to insert before: parent_id={} child_id={} before_id={} (entities not found)",
                parent_id,
                child_id,
                before_id
            );
        }
    }
}

/// Clear root children only.
///
/// Must not drain the whole `ReactContext.nodes` map: concurrent React calls
/// `ClearContainer` after creating the new tree but before attaching it to the
/// root, so a full drain despawns the in-flight mount (blank first frame).
fn handle_clear_container(
    commands: &mut Commands,
    context_entity: Entity,
    context: &mut ReactContext,
) {
    let root = context.root.unwrap_or(context_entity);

    // Defer so we can read `Children` after prior spawn/attach commands apply.
    commands.queue(move |world: &mut World| {
        let children: Vec<Entity> = world
            .get_entity(root)
            .ok()
            .and_then(|e| e.get::<Children>().map(|c| c.iter().collect()))
            .unwrap_or_default();

        if children.is_empty() {
            log::debug!("Cleared container: root had no children (no-op)");
            return;
        }

        if let Some(mut ctx) = world.get_mut::<ReactContext>(context_entity) {
            ctx.nodes.retain(|_, entity| !children.contains(entity));
        }

        for entity in children {
            if world.get_entity(entity).is_ok() {
                world.entity_mut(entity).despawn();
            }
        }
        log::debug!("Cleared container: despawned root children only");
    });
}

fn apply_visual_style(entity_commands: &mut EntityCommands, style_props: &StyleProps) {
    match style_props.background_color.as_deref().and_then(parse_color) {
        Some(mut color) => {
            if let Some(opacity) = style_opacity(style_props) {
                color.set_alpha(opacity);
            }
            entity_commands.insert(BackgroundColor(color));
        }
        None => {
            // Opacity without an explicit background still needs a carrier color.
            if let Some(opacity) = style_opacity(style_props) {
                let mut color = Color::WHITE;
                color.set_alpha(opacity);
                entity_commands.insert(BackgroundColor(color));
            }
        }
    }

    if let Some(border_color) = style_to_border_color(style_props) {
        entity_commands.insert(border_color);
    }
    if let Some(radius) = style_to_border_radius(style_props) {
        entity_commands.insert(radius);
    }
    if let Some(shadow) = style_to_box_shadow(style_props) {
        entity_commands.insert(shadow);
    }
    if let Some(gradient) = style_to_background_gradient(style_props) {
        entity_commands.insert(gradient);
    }
    if let Some(d) = style_props.display.as_deref()
        && d.eq_ignore_ascii_case("none")
    {
        entity_commands.insert(Visibility::Hidden);
    }
}

fn apply_visual_style_commands(commands: &mut Commands, entity: Entity, style_props: &StyleProps) {
    match style_props.background_color.as_deref().and_then(parse_color) {
        Some(mut color) => {
            if let Some(opacity) = style_opacity(style_props) {
                color.set_alpha(opacity);
            }
            commands.entity(entity).insert(BackgroundColor(color));
        }
        None => {
            if style_opacity(style_props).is_none() {
                commands.entity(entity).remove::<BackgroundColor>();
            } else if let Some(opacity) = style_opacity(style_props) {
                let mut color = Color::WHITE;
                color.set_alpha(opacity);
                commands.entity(entity).insert(BackgroundColor(color));
            }
        }
    }

    match style_to_border_color(style_props) {
        Some(border_color) => {
            commands.entity(entity).insert(border_color);
        }
        None => {
            commands.entity(entity).remove::<BorderColor>();
        }
    }
    match style_to_border_radius(style_props) {
        Some(radius) => {
            commands.entity(entity).insert(radius);
        }
        None => {
            commands.entity(entity).remove::<BorderRadius>();
        }
    }
    match style_to_box_shadow(style_props) {
        Some(shadow) => {
            commands.entity(entity).insert(shadow);
        }
        None => {
            commands.entity(entity).remove::<BoxShadow>();
        }
    }
    match style_to_background_gradient(style_props) {
        Some(gradient) => {
            commands.entity(entity).insert(gradient);
        }
        None => {
            commands.entity(entity).remove::<BackgroundGradient>();
        }
    }

    match style_props.display.as_deref() {
        Some(d) if d.eq_ignore_ascii_case("none") => {
            commands.entity(entity).insert(Visibility::Hidden);
        }
        _ => {
            commands.entity(entity).remove::<Visibility>();
        }
    }
}

fn apply_text_style(
    cmd: &mut EntityCommands,
    style_props: &StyleProps,
    asset_server: &AssetServer,
) {
    if let Some(ref color_str) = style_props.color
        && let Some(mut color) = parse_color(color_str)
    {
        if let Some(opacity) = style_opacity(style_props) {
            color.set_alpha(opacity);
        }
        cmd.insert(TextColor(color));
    }

    let mut text_font = TextFont::default();
    let mut has_font = false;
    if let Some(ref font_size) = style_props.font_size
        && let Val::Px(px) = parse_val(&font_size.0)
    {
        text_font.font_size = px;
        has_font = true;
    }
    if let Some(path) = style_font_family(style_props) {
        text_font.font = asset_server.load(path);
        has_font = true;
    }
    if let Some(lh) = style_line_height(style_props) {
        text_font.line_height = lh;
        has_font = true;
    }
    if has_font {
        cmd.insert(text_font);
    }

    if let Some(justify) = style_text_align(style_props) {
        cmd.insert(TextLayout::new_with_justify(justify));
    }
}

fn apply_text_style_commands(
    commands: &mut Commands,
    entity: Entity,
    style_props: &StyleProps,
    asset_server: &AssetServer,
) {
    match style_props.color.as_deref().and_then(parse_color) {
        Some(mut color) => {
            if let Some(opacity) = style_opacity(style_props) {
                color.set_alpha(opacity);
            }
            commands.entity(entity).insert(TextColor(color));
        }
        None => {
            commands.entity(entity).remove::<TextColor>();
        }
    }

    let mut text_font = TextFont::default();
    let mut has_font = false;
    if let Some(font_size) = style_props.font_size.as_ref()
        && let Val::Px(px) = parse_val(&font_size.0)
    {
        text_font.font_size = px;
        has_font = true;
    }
    if let Some(path) = style_font_family(style_props) {
        text_font.font = asset_server.load(path);
        has_font = true;
    }
    if let Some(lh) = style_line_height(style_props) {
        text_font.line_height = lh;
        has_font = true;
    }
    if has_font {
        commands.entity(entity).insert(text_font);
    } else {
        commands.entity(entity).remove::<TextFont>();
    }

    match style_text_align(style_props) {
        Some(justify) => {
            commands
                .entity(entity)
                .insert(TextLayout::new_with_justify(justify));
        }
        None => {
            commands.entity(entity).remove::<TextLayout>();
        }
    }
}
