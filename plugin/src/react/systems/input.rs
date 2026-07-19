use bevy::ecs::archetype::Archetypes;
use bevy::ecs::component::Components;
use bevy::ecs::entity::Entities;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::input::ButtonState;
use bevy::picking::hover::HoverMap;
use bevy::prelude::*;
use bevy::ui::{OverflowAxis, RelativeCursorPosition, ScrollPosition};
use serde_json::{Value, json};

use crate::js_bevy::JsClientResource;
use crate::react::event_queue::{
    keyboard_modifiers, logical_key_to_string, pointer_payload, scroll_payload, wheel_payload,
    FLUSH_EVENTS_SCRIPT, ReactEventQueue,
};
use crate::react::systems::{Focusable, FocusedNode, ReactNode, ReactRoot};

/// Bevy-side request to focus a React node by id (programmatic focus API).
///
/// Other Bevy systems can write this message; the React input systems apply it
/// and emit `focus`/`blur` to JS. A JS→Rust bridge is not wired yet
/// (would need a native `__react_request_focus`).
#[derive(Message, Clone, Debug)]
pub struct RequestReactFocus {
    pub node_id: u64,
    /// Optional root id; when omitted the input system resolves it from the entity tree.
    pub root_id: Option<String>,
}

/// Bevy-side request to clear keyboard focus.
#[derive(Message, Clone, Debug, Default)]
pub struct RequestReactBlur;

/// Component tracking the previous interaction state for hover / press detection
#[derive(Component, Default)]
pub struct PreviousInteraction(pub Interaction);

/// Line-height used when converting `MouseScrollUnit::Line` to pixels.
const SCROLL_LINE_HEIGHT: f32 = 20.0;

/// Handle UI interactions and enqueue events for the native JS dispatcher
pub fn handle_input_interactions(
    mut query: Query<(
        &Interaction,
        &ReactNode,
        Entity,
        Option<&Focusable>,
        Option<&Button>,
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

    let window_cursor = windows.iter().find_map(|w| w.cursor_position());

    for (interaction, react_node, entity, focusable, button, prev, relative) in &mut query {
        let prev_interaction = prev.as_ref().map(|p| p.0).unwrap_or(Interaction::None);

        if *interaction == prev_interaction {
            continue;
        }

        let Some(root_id) = find_root_id(entity, &parents, &roots) else {
            log::warn!("handle_input_interactions: No ReactRoot found");
            continue;
        };

        let node_id = react_node.node_id;
        let payload = pointer_payload(relative, window_cursor);
        let can_focus = is_focus_target(focusable, button);

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

        // Focus on press for focus targets; blur when pressing a non-focusable node
        if is_pressed && !was_pressed {
            if can_focus {
                apply_focus(
                    &mut focused,
                    &event_queue,
                    entity,
                    node_id,
                    root_id.clone(),
                );
            } else {
                // Click on a non-focusable React node → blur (click-outside within UI)
                clear_focus(&mut focused, &event_queue);
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

/// Blur when the primary mouse button is pressed over empty space (no React UI hit).
pub fn handle_click_outside_blur(
    mouse: Res<ButtonInput<MouseButton>>,
    interactions: Query<&Interaction, With<ReactNode>>,
    event_queue: Option<Res<ReactEventQueue>>,
    mut focused: ResMut<FocusedNode>,
) {
    let Some(event_queue) = event_queue else {
        return;
    };

    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }

    let any_over_ui = interactions
        .iter()
        .any(|i| !matches!(*i, Interaction::None));

    if !any_over_ui {
        clear_focus(&mut focused, &event_queue);
    }
}

/// Apply mouse-wheel deltas to `overflow: scroll` nodes via [`ScrollPosition`],
/// and enqueue `wheel` / `scroll` events for the hovered React subtree.
pub fn handle_wheel_scroll(
    mut mouse_wheel_reader: MessageReader<MouseWheel>,
    hover_map: Res<HoverMap>,
    keyboard: Res<ButtonInput<KeyCode>>,
    event_queue: Option<Res<ReactEventQueue>>,
    parents: Query<&ChildOf>,
    roots: Query<&ReactRoot>,
    react_nodes: Query<&ReactNode>,
    nodes: Query<&Node>,
    computed: Query<&ComputedNode>,
    mut scroll_positions: Query<&mut ScrollPosition>,
    mut commands: Commands,
) {
    let Some(event_queue) = event_queue else {
        return;
    };

    for mouse_wheel in mouse_wheel_reader.read() {
        let mut delta = match mouse_wheel.unit {
            MouseScrollUnit::Line => {
                Vec2::new(mouse_wheel.x, mouse_wheel.y) * SCROLL_LINE_HEIGHT
            }
            MouseScrollUnit::Pixel => Vec2::new(mouse_wheel.x, mouse_wheel.y),
        };

        // Invert so positive wheel-up moves content down (DOM / Bevy scroll convention).
        delta = -delta;

        if keyboard.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
            std::mem::swap(&mut delta.x, &mut delta.y);
        }

        if delta == Vec2::ZERO {
            continue;
        }

        let mut hovered: Vec<Entity> = hover_map
            .values()
            .flat_map(|pointer_map| pointer_map.keys().copied())
            .collect();
        hovered.sort();
        hovered.dedup();

        for start in hovered {
            let mut remaining = delta;
            let mut current = Some(start);

            while let Some(entity) = current {
                if remaining == Vec2::ZERO {
                    break;
                }

                let Ok(node) = nodes.get(entity) else {
                    current = parents.get(entity).ok().map(|c| c.parent());
                    continue;
                };

                let can_scroll_x = node.overflow.x == OverflowAxis::Scroll;
                let can_scroll_y = node.overflow.y == OverflowAxis::Scroll;

                if !can_scroll_x && !can_scroll_y {
                    current = parents.get(entity).ok().map(|c| c.parent());
                    continue;
                }

                let Ok(computed_node) = computed.get(entity) else {
                    current = parents.get(entity).ok().map(|c| c.parent());
                    continue;
                };

                let max_offset = ((computed_node.content_size() - computed_node.size())
                    * computed_node.inverse_scale_factor())
                .max(Vec2::ZERO);

                let mut applied = Vec2::ZERO;
                let next_pos = match scroll_positions.get_mut(entity) {
                    Ok(mut scroll_pos) => {
                        let mut pos = **scroll_pos;
                        if can_scroll_x && remaining.x != 0.0 {
                            let before = pos.x;
                            pos.x = (pos.x + remaining.x).clamp(0.0, max_offset.x);
                            applied.x = pos.x - before;
                            remaining.x -= applied.x;
                        }
                        if can_scroll_y && remaining.y != 0.0 {
                            let before = pos.y;
                            pos.y = (pos.y + remaining.y).clamp(0.0, max_offset.y);
                            applied.y = pos.y - before;
                            remaining.y -= applied.y;
                        }
                        if applied != Vec2::ZERO {
                            **scroll_pos = pos;
                        }
                        pos
                    }
                    Err(_) => {
                        let mut pos = Vec2::ZERO;
                        if can_scroll_x && remaining.x != 0.0 {
                            let before = pos.x;
                            pos.x = (pos.x + remaining.x).clamp(0.0, max_offset.x);
                            applied.x = pos.x - before;
                            remaining.x -= applied.x;
                        }
                        if can_scroll_y && remaining.y != 0.0 {
                            let before = pos.y;
                            pos.y = (pos.y + remaining.y).clamp(0.0, max_offset.y);
                            applied.y = pos.y - before;
                            remaining.y -= applied.y;
                        }
                        if applied != Vec2::ZERO {
                            // Lazily attach ScrollPosition (render path does not insert it).
                            commands.entity(entity).insert(ScrollPosition(pos));
                        }
                        pos
                    }
                };

                if let Ok(react_node) = react_nodes.get(entity)
                    && let Some(root_id) = find_root_id(entity, &parents, &roots) {
                        let node_id = react_node.node_id;
                        event_queue.push_event(
                            root_id.clone(),
                            node_id,
                            "wheel",
                            wheel_payload(delta, mouse_wheel.unit),
                        );
                        if applied != Vec2::ZERO {
                            event_queue.push_event(
                                root_id,
                                node_id,
                                "scroll",
                                scroll_payload(next_pos.x, next_pos.y, applied),
                            );
                        }
                    }

                current = parents.get(entity).ok().map(|c| c.parent());
            }
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

fn is_focus_target(focusable: Option<&Focusable>, button: Option<&Button>) -> bool {
    focusable.is_some() || button.is_some()
}

fn apply_focus(
    focused: &mut FocusedNode,
    event_queue: &ReactEventQueue,
    entity: Entity,
    node_id: u64,
    root_id: String,
) {
    if focused.node_id == Some(node_id) {
        return;
    }

    if let Some(old_id) = focused.node_id {
        let blur_root = focused
            .module_name
            .clone()
            .unwrap_or_else(|| root_id.clone());
        event_queue.push_event(blur_root, old_id, "blur", Value::Null);
    }

    focused.node_id = Some(node_id);
    focused.entity = Some(entity);
    focused.module_name = Some(root_id.clone());
    event_queue.push_event(root_id, node_id, "focus", Value::Null);
}

fn clear_focus(focused: &mut FocusedNode, event_queue: &ReactEventQueue) {
    let Some(old_id) = focused.node_id.take() else {
        focused.entity = None;
        focused.module_name = None;
        return;
    };

    let blur_root = focused.module_name.take().unwrap_or_default();
    focused.entity = None;

    if !blur_root.is_empty() {
        event_queue.push_event(blur_root, old_id, "blur", Value::Null);
    }
}

/// Apply [`RequestReactFocus`] / [`RequestReactBlur`] messages (Bevy programmatic focus API).
pub fn apply_focus_requests(
    mut focus_requests: MessageReader<RequestReactFocus>,
    mut blur_requests: MessageReader<RequestReactBlur>,
    event_queue: Option<Res<ReactEventQueue>>,
    mut focused: ResMut<FocusedNode>,
    targets: Query<(Entity, &ReactNode, Option<&Focusable>, Option<&Button>)>,
    parents: Query<&ChildOf>,
    roots: Query<&ReactRoot>,
) {
    let Some(event_queue) = event_queue else {
        return;
    };

    for _ in blur_requests.read() {
        clear_focus(&mut focused, &event_queue);
    }

    for request in focus_requests.read() {
        let Some((entity, react_node, focusable, button)) = targets
            .iter()
            .find(|(_, rn, _, _)| rn.node_id == request.node_id)
        else {
            log::warn!(
                "RequestReactFocus: no entity for node_id={}",
                request.node_id
            );
            continue;
        };

        if !is_focus_target(focusable, button) {
            log::warn!(
                "RequestReactFocus: node_id={} is not a focus target",
                request.node_id
            );
            continue;
        }

        let root_id = request
            .root_id
            .clone()
            .or_else(|| find_root_id(entity, &parents, &roots));

        let Some(root_id) = root_id else {
            log::warn!(
                "RequestReactFocus: no ReactRoot for node_id={}",
                request.node_id
            );
            continue;
        };

        apply_focus(
            &mut focused,
            &event_queue,
            entity,
            react_node.node_id,
            root_id,
        );
    }
}

/// Collect focus targets in depth-first tree order under each React root.
fn collect_focus_targets(
    roots: &Query<(Entity, &ReactRoot)>,
    children: &Query<&Children>,
    targets: &Query<(Entity, &ReactNode, Option<&Focusable>, Option<&Button>)>,
) -> Vec<(Entity, u64, String, bool)> {
    let mut out = Vec::new();

    fn walk(
        entity: Entity,
        root_id: &str,
        children: &Query<&Children>,
        targets: &Query<(Entity, &ReactNode, Option<&Focusable>, Option<&Button>)>,
        out: &mut Vec<(Entity, u64, String, bool)>,
    ) {
        if let Ok((e, rn, focusable, button)) = targets.get(entity)
            && is_focus_target(focusable, button) {
                // `is_text_like`: Focusable marker (text inputs) — arrows stay for editing.
                let is_text_like = focusable.is_some();
                out.push((e, rn.node_id, root_id.to_string(), is_text_like));
            }

        if let Ok(kids) = children.get(entity) {
            for child in kids.iter() {
                walk(child, root_id, children, targets, out);
            }
        }
    }

    for (root_entity, root) in roots.iter() {
        walk(root_entity, &root.id, children, targets, &mut out);
    }

    out
}

fn navigate_focus(
    focused: &mut FocusedNode,
    event_queue: &ReactEventQueue,
    targets: &[(Entity, u64, String, bool)],
    forward: bool,
) {
    if targets.is_empty() {
        return;
    }

    let next_index = match focused.node_id {
        Some(current) => match targets.iter().position(|(_, id, _, _)| *id == current) {
            Some(idx) => {
                if forward {
                    (idx + 1) % targets.len()
                } else if idx == 0 {
                    targets.len() - 1
                } else {
                    idx - 1
                }
            }
            None => {
                // Focused id not in list — jump to first/last
                if forward {
                    0
                } else {
                    targets.len() - 1
                }
            }
        },
        None => {
            if forward {
                0
            } else {
                targets.len() - 1
            }
        }
    };

    let (entity, node_id, root_id, _) = &targets[next_index];
    apply_focus(
        focused,
        event_queue,
        *entity,
        *node_id,
        root_id.clone(),
    );
}

/// Handle keyboard input: Tab/arrow focus navigation, then forward keys to the focused node.
pub fn handle_keyboard_input(
    mut keyboard_events: MessageReader<KeyboardInput>,
    mut focused: ResMut<FocusedNode>,
    event_queue: Option<Res<ReactEventQueue>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    roots: Query<(Entity, &ReactRoot)>,
    children: Query<&Children>,
    targets: Query<(Entity, &ReactNode, Option<&Focusable>, Option<&Button>)>,
) {
    let Some(event_queue) = event_queue else {
        return;
    };

    let focus_list = collect_focus_targets(&roots, &children, &targets);
    let modifiers = keyboard_modifiers(&keyboard);

    for event in keyboard_events.read() {
        let key = logical_key_to_string(&event.logical_key);
        let is_pressed = event.state == ButtonState::Pressed;

        // Tab / Shift+Tab — always move focus among targets (consume, do not forward).
        if is_pressed && key == "Tab" && !event.repeat {
            let forward = !keyboard.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
            navigate_focus(&mut focused, &event_queue, &focus_list, forward);
            continue;
        }

        // Arrow keys navigate when focus is not on a text-like Focusable (or nothing focused).
        if is_pressed
            && matches!(
                key.as_str(),
                "ArrowDown" | "ArrowRight" | "ArrowUp" | "ArrowLeft"
            )
            && !event.repeat
        {
            let on_text = focused
                .node_id
                .and_then(|id| {
                    focus_list
                        .iter()
                        .find(|(_, nid, _, _)| *nid == id)
                        .map(|(_, _, _, is_text)| *is_text)
                })
                .unwrap_or(false);

            if !on_text {
                let forward = matches!(key.as_str(), "ArrowDown" | "ArrowRight");
                navigate_focus(&mut focused, &event_queue, &focus_list, forward);
                continue;
            }
        }

        let Some(focused_node_id) = focused.node_id else {
            continue;
        };

        // Legacy field: stores root_id for the focused element
        let Some(ref root_id) = focused.module_name else {
            continue;
        };

        let event_type = match event.state {
            ButtonState::Pressed => "keydown",
            ButtonState::Released => "keyup",
        };

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
            if let Some(entity_location) = entities.get(entity)
                && let Some(archetype) = archetypes.get(entity_location.archetype_id)
            {
                for component in archetype.components() {
                    if let Some(info) = components.get_info(*component) {
                        println!("\tComponent: {}", info.name());
                    }
                }
            }
        }
    }
}
