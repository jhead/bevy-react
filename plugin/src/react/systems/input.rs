use bevy::ecs::archetype::Archetypes;
use bevy::ecs::component::Components;
use bevy::ecs::entity::Entities;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::ButtonState;
use bevy::prelude::*;

use crate::js_bevy::JsClientResource;
use crate::react::systems::{Focusable, FocusedNode, ReactNode, ReactScriptSource};

/// Handle UI interactions and dispatch events to JS
pub fn handle_input_interactions(
    query: Query<(&Interaction, &ReactNode, Entity, Option<&Focusable>), Changed<Interaction>>,
    js_client: Option<Res<JsClientResource>>,
    mut focused: ResMut<FocusedNode>,
    parents: Query<&ChildOf>,
    scripts: Query<&ReactScriptSource>,
) {
    let Some(js_client) = js_client else {
        return;
    };

    for (interaction, react_node, entity, focusable) in &query {
        let Some(module_name) = find_module_name(entity, &parents, &scripts) else {
            log::warn!("handle_input_interactions: No React script source found");
            continue;
        };

        if *interaction == Interaction::Pressed {
            // Handle focus for focusable elements
            if focusable.is_some() {
                let old_focused = focused.node_id;
                
                // Blur previous element if different
                if let Some(old_id) = old_focused {
                    if old_id != react_node.node_id {
                        // Use the old module name if available, otherwise use current
                        let blur_module = focused.module_name.as_ref().unwrap_or(&module_name);
                        js_client.execute(format!(
                            r#"
                            (async () => {{
                                try {{
                                    const mod = await import('{blur_module}');
                                    mod.default.dispatchEvent({old_id}, 'blur');
                                }} catch (err) {{
                                    console.error("Failed to dispatch blur event:", err);
                                }}
                            }})()
                            "#,
                        ));
                    }
                }
                
                // Focus the new element and cache the module name
                focused.node_id = Some(react_node.node_id);
                focused.entity = Some(entity);
                focused.module_name = Some(module_name.clone());
                
                js_client.execute(format!(
                    r#"
                    (async () => {{
                        try {{
                            const mod = await import('{module_name}');
                            mod.default.dispatchEvent({node_id}, 'focus');
                        }} catch (err) {{
                            console.error("Failed to dispatch focus event:", err);
                        }}
                    }})()
                    "#,
                    module_name = module_name,
                    node_id = react_node.node_id
                ));
            }

            log::debug!(
                "Node clicked: id={}, module={}",
                react_node.node_id,
                module_name
            );
            js_client.execute(format!(
                r#"
                (async () => {{
                    try {{
                        const mod = await import('{module_name}');
                        mod.default.dispatchEvent({node_id}, 'click');
                    }} catch (err) {{
                        console.error("Failed to dispatch event:", err);
                    }}
                }})()
            "#,
                module_name = module_name,
                node_id = react_node.node_id
            ));
        }
    }
}

/// Find the module name by traversing up the entity hierarchy
fn find_module_name(
    entity: Entity,
    parents: &Query<&ChildOf>,
    scripts: &Query<&ReactScriptSource>,
) -> Option<String> {
    let mut current = Some(entity);

    while let Some(e) = current {
        if let Ok(script) = scripts.get(e) {
            return Some(script.module_name.clone());
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
    js_client: Option<Res<JsClientResource>>,
) {
    let Some(js_client) = js_client else {
        return;
    };

    let Some(focused_node_id) = focused.node_id else {
        return;
    };

    let Some(ref module_name) = focused.module_name else {
        return;
    };

    for event in keyboard_events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }

        // Convert the key to a string representation
        let key_str = format!("{:?}", event.key_code);
        
        log::debug!(
            "Keyboard event: key={}, focused_node={}",
            key_str,
            focused_node_id
        );

        // Escape JSON special characters in key string
        let key_escaped = key_str.replace('\\', "\\\\").replace('"', "\\\"");

        js_client.execute(format!(
            r#"
            (async () => {{
                try {{
                    const mod = await import('{module_name}');
                    mod.default.dispatchEvent({node_id}, 'keydown', {{ key: "{key}" }});
                }} catch (err) {{
                    console.error("Failed to dispatch keydown event:", err);
                }}
            }})()
            "#,
            module_name = module_name,
            node_id = focused_node_id,
            key = key_escaped
        ));
    }
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
