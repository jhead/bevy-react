//! Headless Bevy UI widgets (`bevy_ui_widgets`) bridged to React.
//!
//! Host owns interaction (drag, activate, focus). React owns look via styles and
//! receives `change` / `click` events through [`ReactEventQueue`].

use bevy::input_focus::InputDispatchPlugin;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{Checked, InteractionDisabled, Pressed};
use bevy::ui_widgets::{
    observe, Activate, Button as WidgetButton, Checkbox, CoreSliderDragState, Slider, SliderRange,
    SliderStep, SliderThumb, SliderValue, UiWidgetsPlugins, ValueChange,
};
use serde::Deserialize;
use serde_json::json;

use crate::react::event_queue::ReactEventQueue;
use crate::react::systems::{process_react_messages, ReactNode, ReactRoot};

/// Widget props parsed from the React props JSON (not style.rs).
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WidgetProps {
    value: Option<f32>,
    min: Option<f32>,
    max: Option<f32>,
    step: Option<f32>,
    checked: Option<bool>,
    disabled: Option<bool>,
    /// `"horizontal"` (default) or `"vertical"`.
    ///
    /// Bevy 0.17's headless [`Slider`] drag/keyboard math is horizontal-only
    /// (`// TODO: vertical` upstream). Vertical still positions the thumb on the
    /// Y axis for layout demos; pointer drag remains width-based.
    orientation: Option<String>,
}

fn parse_widget_props(props_json: &str) -> WidgetProps {
    serde_json::from_str(props_json).unwrap_or_default()
}

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
        current = parents.get(e).ok().map(|c| c.parent());
    }
    None
}

fn apply_disabled(entity_commands: &mut EntityCommands, disabled: bool) {
    if disabled {
        entity_commands.insert(InteractionDisabled);
    } else {
        entity_commands.remove::<InteractionDisabled>();
    }
}

fn is_vertical_orientation(orientation: Option<&str>) -> bool {
    orientation.is_some_and(|o| o.eq_ignore_ascii_case("vertical"))
}

/// Marker storing slider orientation for thumb sync (Bevy Slider has no vertical API yet).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
enum SliderOrientation {
    #[default]
    Horizontal,
    Vertical,
}

/// Register widget plugins required for headless controls.
pub fn add_widget_plugins(app: &mut App) {
    app.add_plugins((UiWidgetsPlugins, InputDispatchPlugin))
        .add_systems(
            Update,
            sync_slider_thumb_positions.after(process_react_messages),
        );
}

/// Attach headless [`WidgetButton`] alongside the legacy UI `Button` marker.
///
/// Pointer clicks still flow through Interaction → `input.rs`. [`Activate`] from
/// pointer arrives while [`Pressed`] is still present (deferred remove) and is
/// skipped here; keyboard Enter/Space Activate has no [`Pressed`] and becomes
/// React `click` (no double-fire).
pub fn insert_button_widget(entity_commands: &mut EntityCommands) {
    entity_commands.insert((
        WidgetButton,
        Hovered::default(),
        observe(forward_button_activate),
    ));
}

/// Attach headless [`Slider`] + range/value/step and forward [`ValueChange<f32>`].
pub fn insert_slider_widget(entity_commands: &mut EntityCommands, props_json: &str) {
    let props = parse_widget_props(props_json);
    let min = props.min.unwrap_or(0.0);
    let max = props.max.unwrap_or(1.0);
    let value = props.value.unwrap_or(min);
    let step = props.step.unwrap_or(1.0);
    let orientation = if is_vertical_orientation(props.orientation.as_deref()) {
        SliderOrientation::Vertical
    } else {
        SliderOrientation::Horizontal
    };

    entity_commands.insert((
        Slider::default(),
        SliderValue(value),
        SliderRange::new(min, max),
        SliderStep(step),
        orientation,
        Hovered::default(),
        observe(forward_slider_change),
    ));

    apply_disabled(entity_commands, props.disabled.unwrap_or(false));
}

/// Attach headless [`Checkbox`] and forward [`ValueChange<bool>`].
pub fn insert_checkbox_widget(entity_commands: &mut EntityCommands, props_json: &str) {
    let props = parse_widget_props(props_json);

    entity_commands.insert((Checkbox, Hovered::default(), observe(forward_checkbox_change)));

    if props.checked.unwrap_or(false) {
        entity_commands.insert(Checked);
    }

    apply_disabled(entity_commands, props.disabled.unwrap_or(false));
}

/// Mark a descendant as the slider thumb (required for drag hit-testing).
pub fn insert_slider_thumb(entity_commands: &mut EntityCommands) {
    entity_commands.insert(SliderThumb);
}

/// Sync widget state components from an updated React props JSON.
pub fn sync_widget_props(commands: &mut Commands, entity: Entity, props_json: String) {
    commands.queue(move |world: &mut World| {
        let props = parse_widget_props(&props_json);
        let Ok(mut entity_mut) = world.get_entity_mut(entity) else {
            return;
        };

        if entity_mut.contains::<Slider>() {
            let min = props.min.unwrap_or_else(|| {
                entity_mut
                    .get::<SliderRange>()
                    .map(|r| r.start())
                    .unwrap_or(0.0)
            });
            let max = props.max.unwrap_or_else(|| {
                entity_mut
                    .get::<SliderRange>()
                    .map(|r| r.end())
                    .unwrap_or(1.0)
            });
            let step = props.step.unwrap_or_else(|| {
                entity_mut
                    .get::<SliderStep>()
                    .map(|s| s.0)
                    .unwrap_or(1.0)
            });
            let dragging = entity_mut
                .get::<CoreSliderDragState>()
                .map(|d| d.dragging)
                .unwrap_or(false);

            let orientation = if is_vertical_orientation(props.orientation.as_deref()) {
                SliderOrientation::Vertical
            } else {
                SliderOrientation::Horizontal
            };

            entity_mut.insert((
                SliderRange::new(min, max),
                SliderStep(step),
                orientation,
            ));

            // Avoid stomping the live drag value with a stale controlled prop.
            if !dragging {
                let value = props.value.unwrap_or_else(|| {
                    entity_mut
                        .get::<SliderValue>()
                        .map(|v| v.0)
                        .unwrap_or(min)
                });
                entity_mut.insert(SliderValue(value));
            }

            if props.disabled.unwrap_or(false) {
                entity_mut.insert(InteractionDisabled);
            } else {
                entity_mut.remove::<InteractionDisabled>();
            }
        }

        if entity_mut.contains::<Checkbox>() {
            let checked = props.checked.unwrap_or(false);
            if checked {
                entity_mut.insert(Checked);
            } else {
                entity_mut.remove::<Checked>();
            }

            if props.disabled.unwrap_or(false) {
                entity_mut.insert(InteractionDisabled);
            } else {
                entity_mut.remove::<InteractionDisabled>();
            }
        }
    });
}

/// Keyboard Activate → React `click`. Pointer Activate is ignored (see [`insert_button_widget`]).
fn forward_button_activate(
    activate: On<Activate>,
    pressed: Query<Has<Pressed>>,
    event_queue: Res<ReactEventQueue>,
    nodes: Query<&ReactNode>,
    parents: Query<&ChildOf>,
    roots: Query<&ReactRoot>,
) {
    // Pointer path: Activate is triggered while `Pressed` is still visible to this
    // query (Release's remove is deferred). Keyboard Enter/Space never inserts Pressed.
    if pressed.get(activate.entity).unwrap_or(false) {
        return;
    }

    let Ok(node) = nodes.get(activate.entity) else {
        return;
    };
    let Some(root_id) = find_root_id(activate.entity, &parents, &roots) else {
        return;
    };
    event_queue.push_event(root_id, node.node_id, "click", json!({}));
}

fn forward_slider_change(
    change: On<ValueChange<f32>>,
    mut commands: Commands,
    event_queue: Res<ReactEventQueue>,
    nodes: Query<&ReactNode>,
    parents: Query<&ChildOf>,
    roots: Query<&ReactRoot>,
) {
    // Keep host value in sync during drag (external React state catches up via events).
    commands
        .entity(change.source)
        .insert(SliderValue(change.value));

    let Ok(node) = nodes.get(change.source) else {
        return;
    };
    let Some(root_id) = find_root_id(change.source, &parents, &roots) else {
        return;
    };
    event_queue.push_event(
        root_id,
        node.node_id,
        "change",
        json!({ "value": change.value }),
    );
}

fn forward_checkbox_change(
    change: On<ValueChange<bool>>,
    mut commands: Commands,
    event_queue: Res<ReactEventQueue>,
    nodes: Query<&ReactNode>,
    parents: Query<&ChildOf>,
    roots: Query<&ReactRoot>,
) {
    if change.value {
        commands.entity(change.source).insert(Checked);
    } else {
        commands.entity(change.source).remove::<Checked>();
    }

    let Ok(node) = nodes.get(change.source) else {
        return;
    };
    let Some(root_id) = find_root_id(change.source, &parents, &roots) else {
        return;
    };
    event_queue.push_event(
        root_id,
        node.node_id,
        "change",
        json!({ "value": change.value }),
    );
}

/// Position slider thumbs from host [`SliderValue`] (smooth drag without waiting on React).
///
/// Travel is expected to be inset by thumb size on the trailing edge so percent
/// positions match Bevy's drag math (full track minus thumb).
fn sync_slider_thumb_positions(
    sliders: Query<
        (Entity, &SliderValue, &SliderRange, Option<&SliderOrientation>),
        (With<Slider>, With<ReactNode>),
    >,
    children: Query<&Children>,
    mut thumbs: Query<&mut Node, With<SliderThumb>>,
) {
    for (slider_ent, value, range, orientation) in &sliders {
        let pct = range.thumb_position(value.0) * 100.0;
        let vertical = orientation == Some(&SliderOrientation::Vertical);
        for child in children.iter_descendants(slider_ent) {
            if let Ok(mut thumb_node) = thumbs.get_mut(child) {
                if vertical {
                    // CSS-like: 0% at top → min; grow downward with value.
                    thumb_node.top = Val::Percent(pct);
                    thumb_node.left = Val::Auto;
                    thumb_node.right = Val::Auto;
                    thumb_node.bottom = Val::Auto;
                } else {
                    thumb_node.left = Val::Percent(pct);
                    thumb_node.right = Val::Auto;
                    // Keep author-provided vertical centering (`top`) intact.
                }
            }
        }
    }
}
