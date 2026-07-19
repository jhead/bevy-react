use bevy::ecs::archetype::Archetypes;
use bevy::ecs::component::Components;
use bevy::ecs::entity::Entities;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;
use serde_json::{Value, json};

use crate::js_bevy::JsClientResource;
use crate::react::event_queue::{
    keyboard_modifiers, logical_key_to_string, pointer_payload, FLUSH_EVENTS_SCRIPT, ReactEventQueue,
};
use crate::react::systems::{Focusable, FocusedNode, ReactNode, ReactRoot};

/// Component tracking the previous interaction state for hover / press detection
#[derive(Component, Default)]
pub struct PreviousInteraction(pub Interaction);

/// Handle UI interactions and enqueue events for the native JS dispatcher
pub fn handle_input_interactions(
    mut query: Query<(
        &Interaction,
        &ReactNode,
        Entity,
        Option<&Focusable>,
        Option<&mut PreviousInteraction>,
        Option<&RelativeCursorPosition>,
    )>,
    event_queue: Option<Res<ReactEventQueue>>,
    mut focused: ResMut<FocusedNode>,
    parents: Query<&ChildOf>,
    roots: Query<&ReactRoot>,
    windows: Query<&Window>,
    mut commands: Commands,
) {
    let Some(event_queue) = event_queue else {
        return;
    };

    let window_cursor = windows
        .iter()
        .find_map(|w| w.cursor_position());

    for (interaction, react_node, entity, focusable, prev, relative) in &mut query {
        let prev_interaction = prev
            .as_ref()
            .map(|p| p.0)
            .unwrap_or(Interaction::None);

        if *interaction == prev_interaction {
            continue;
        }

        let Some(root_id) = find_root_id(entity, &parents, &roots) else {
            log::warn!("handle_input_interactions: No ReactRoot found");
            continue;
        };

        let node_id = react_node.node_id;
        let payload = pointer_payload(relative, window_cursor);

        // Ensure relative cursor tracking for subsequent pointer events
        if relative.is_none() {
            commands
                .entity(entity)
                .insert(RelativeCursorPosition::default());
        }

        let was_hovered = matches!(
            prev_interaction,
            Interaction::Hovered | Interaction::Pressed
        );
        let is_hovered = matches!(
            *interaction,
            Interaction::Hovered | Interaction::Pressed
        );
        let was_pressed = prev_interaction == Interaction::Pressed;
        let is_pressed = *interaction == Interaction::Pressed;

        // Focus on press for focusable elements
        if is_pressed && !was_pressed {
            if focusable.is_some() {
                let old_focused = focused.node_id;

                if let Some(old_id) = old_focused {
                    if old_id != node_id {
                        // `module_name` field stores root_id (legacy field name in FocusedNode)
                        let blur_root = focused
                            .module_name
                            .clone()
                            .unwrap_or_else(|| root_id.clone());
                        event_queue.push_event(blur_root, old_id, "blur", Value::Null);
                    }
                }

                focused.node_id = Some(node_id);
                focused.entity = Some(entity);
                focused.module_name = Some(root_id.clone());

                event_queue.push_event(root_id.clone(), node_id, "focus", Value::Null);
            }

            log::debug!("Node press: id={}, root={}", node_id, root_id);
            event_queue.push_event(root_id.clone(), node_id, "press", payload.clone());
            // Keep existing click-on-press timing
            event_queue.push_event(root_id.clone(), node_id, "click", payload.clone());
        }

        if was_pressed && !is_pressed {
            log::debug!("Node release: id={}, root={}", node_id, root_id);
            event_queue.push_event(root_id.clone(), node_id, "release", payload.clone());
        }

        if !was_hovered && is_hovered {
            event_queue.push_event(root_id.clone(), node_id, "mouseenter", payload.clone());
        } else if was_hovered && !is_hovered {
            event_queue.push_event(root_id.clone(), node_id, "mouseleave", payload);
        }

        if let Some(mut prev) = prev {
            prev.0 = *interaction;
        } else {
            commands
                .entity(entity)
                .insert(PreviousInteraction(*interaction));
        }
    }
}

/// Find the React root id by traversing up the entity hierarchy
fn find_root_id(
    entity: Entity,
    parents: &Query<&ChildOf>,
    roots: &Query<&ReactRoot>,
) -> Option<String> {
    let mut current = Some(entity);

    while let Some(e) = current {
        if let Ok(root) = roots.get(e) {
            return Some(root.id.clone());
        }

        if let Ok(child_of) = parents.get(e) {
            current = Some(child_of.parent());
        } else {
            current = None;
        }
    }
    None
}

/// Handle keyboard input and forward to focused React elements
pub fn handle_keyboard_input(
    mut keyboard_events: MessageReader<KeyboardInput>,
    focused: Res<FocusedNode>,
    event_queue: Option<Res<ReactEventQueue>>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    let Some(event_queue) = event_queue else {
        return;
    };

    let Some(focused_node_id) = focused.node_id else {
        return;
    };

    // Legacy field: stores root_id for the focused element
    let Some(ref root_id) = focused.module_name else {
        return;
    };

    let modifiers = keyboard_modifiers(&keyboard);

    for event in keyboard_events.read() {
        let event_type = match event.state {
            ButtonState::Pressed => "keydown",
            ButtonState::Released => "keyup",
        };

        let key = logical_key_to_string(&event.logical_key);

        let mut payload = modifiers.as_object().cloned().unwrap_or_default();
        payload.insert("key".to_string(), json!(key));
        payload.insert("repeat".to_string(), json!(event.repeat));
        if let Some(text) = event.text.as_ref() {
            payload.insert("text".to_string(), json!(text.to_string()));
        }

        log::debug!(
            "Keyboard event: type={}, key={}, focused_node={}",
            event_type,
            payload.get("key").and_then(|v| v.as_str()).unwrap_or(""),
            focused_node_id
        );

        event_queue.push_event(
            root_id.clone(),
            focused_node_id,
            event_type,
            Value::Object(payload),
        );
    }
}

/// Drain the event queue into JS via a fixed native flush (no eval interpolation).
pub fn flush_react_events(
    event_queue: Option<Res<ReactEventQueue>>,
    js_client: Option<Res<JsClientResource>>,
) {
    let Some(event_queue) = event_queue else {
        return;
    };
    let Some(js_client) = js_client else {
        return;
    };

    if event_queue.is_empty() {
        return;
    }

    js_client.execute(FLUSH_EVENTS_SCRIPT);
}

pub fn inspect(
    keyboard: Res<ButtonInput<KeyCode>>,
    all_entities: Query<Entity>,
    entities: &Entities,
    archetypes: &Archetypes,
    components: &Components,
) {
    if keyboard.just_pressed(KeyCode::F1) {
        for entity in all_entities.iter() {
            println!("Entity: {:?}", entity);
            if let Some(entity_location) = entities.get(entity) {
                if let Some(archetype) = archetypes.get(entity_location.archetype_id) {
                    for component in archetype.components() {
                        if let Some(info) = components.get_info(*component) {
                            println!("\tComponent: {}", info.name());
                        }
                    }
                }
            }
        }
    }
}
