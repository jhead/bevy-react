//! Built-in Bevy UI overlay for [`super::JsRuntimeError`].
//!
//! Spawns a high [`GlobalZIndex`] panel with the latest JS error message + stack.
//! Dismiss with the button or Escape (clears the resource).

use bevy::prelude::*;

use crate::js::JsErrorSource;
use super::JsRuntimeError;

const OVERLAY_Z: i32 = 10_000;

#[derive(Component)]
struct JsErrorOverlayRoot;

#[derive(Component)]
struct JsErrorOverlayMessage;

#[derive(Component)]
struct JsErrorOverlayStack;

#[derive(Component)]
struct JsErrorOverlayDismiss;

/// Register overlay systems (after [`super::plugin::JsRuntimeErrorSyncSet`]).
pub(crate) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (sync_js_error_overlay, dismiss_js_error_overlay)
            .chain()
            .after(super::plugin::JsRuntimeErrorSyncSet),
    );
}

fn sync_js_error_overlay(
    mut commands: Commands,
    runtime_error: Res<JsRuntimeError>,
    overlay: Query<Entity, With<JsErrorOverlayRoot>>,
    mut message_q: Query<&mut Text, (With<JsErrorOverlayMessage>, Without<JsErrorOverlayStack>)>,
    mut stack_q: Query<&mut Text, (With<JsErrorOverlayStack>, Without<JsErrorOverlayMessage>)>,
) {
    let Some(record) = runtime_error.last_error.as_ref() else {
        for entity in &overlay {
            commands.entity(entity).despawn();
        }
        return;
    };

    let title = format_title(record.source);
    let message = format!("{title}\n{}", record.message);
    let stack = record
        .stack
        .clone()
        .unwrap_or_else(|| "(no stack)".to_string());

    if overlay.is_empty() {
        spawn_overlay(&mut commands, &message, &stack);
        return;
    }

    // Update existing overlay text when a new error arrives.
    if runtime_error.is_changed() {
        if let Ok(mut text) = message_q.single_mut() {
            *text = Text::new(message);
        }
        if let Ok(mut text) = stack_q.single_mut() {
            *text = Text::new(stack);
        }
    }
}

fn dismiss_js_error_overlay(
    mut commands: Commands,
    mut runtime_error: ResMut<JsRuntimeError>,
    keyboard: Res<ButtonInput<KeyCode>>,
    buttons: Query<&Interaction, With<JsErrorOverlayDismiss>>,
    overlay: Query<Entity, With<JsErrorOverlayRoot>>,
) {
    if runtime_error.last_error.is_none() {
        return;
    }

    let dismiss_clicked = buttons
        .iter()
        .any(|i| matches!(*i, Interaction::Pressed));
    let escape = keyboard.just_pressed(KeyCode::Escape);

    if !dismiss_clicked && !escape {
        return;
    }

    runtime_error.clear();
    for entity in &overlay {
        commands.entity(entity).despawn();
    }
}

fn format_title(source: JsErrorSource) -> String {
    let label = match source {
        JsErrorSource::Console => "console.error",
        JsErrorSource::UncaughtRejection => "Uncaught promise rejection",
        JsErrorSource::Script => "Script error",
        JsErrorSource::ModuleLoad => "Module load error",
        JsErrorSource::Job => "Job / microtask error",
        JsErrorSource::Panic => "JS engine panic",
        JsErrorSource::React => "React error",
    };
    format!("JS Error — {label}")
}

fn spawn_overlay(commands: &mut Commands, message: &str, stack: &str) {
    commands
        .spawn((
            JsErrorOverlayRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(24.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.72)),
            GlobalZIndex(OVERLAY_Z),
            Interaction::default(),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        width: Val::Px(640.0),
                        max_width: Val::Percent(92.0),
                        max_height: Val::Percent(85.0),
                        padding: UiRect::all(Val::Px(16.0)),
                        row_gap: Val::Px(12.0),
                        border: UiRect::all(Val::Px(2.0)),
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.12, 0.08, 0.08)),
                    BorderColor::all(Color::srgb(0.85, 0.25, 0.25)),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        JsErrorOverlayMessage,
                        Text::new(message.to_string()),
                        TextFont {
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 0.85, 0.85)),
                        Node {
                            flex_shrink: 0.0,
                            ..default()
                        },
                    ));

                    panel.spawn((
                        JsErrorOverlayStack,
                        Text::new(stack.to_string()),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                        Node {
                            flex_shrink: 1.0,
                            ..default()
                        },
                    ));

                    panel
                        .spawn((
                            JsErrorOverlayDismiss,
                            Button,
                            Node {
                                padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)),
                                align_self: AlignSelf::FlexEnd,
                                flex_shrink: 0.0,
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.55, 0.15, 0.15)),
                            BorderColor::all(Color::srgb(0.9, 0.4, 0.4)),
                            BorderRadius::all(Val::Px(4.0)),
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new("Dismiss (Esc)"),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        });
                });
        });
}
