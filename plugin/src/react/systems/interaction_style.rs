//! Host-side interaction styling: merge hover/pressed/focused overrides and
//! apply simple color/opacity transitions without React round-trips.

use bevy::prelude::*;
use bevy::text::TextColor;
use bevy::ui::FocusPolicy;

use crate::react::style::{
    json_to_style, parse_color, resolve_interaction_style, style_opacity, style_pointer_events,
    style_to_background_gradient, style_to_border_color, style_to_border_radius,
    style_to_box_shadow, PointerEvents, StyleProps,
};
use crate::react::systems::types::{
    ColorAnim, FloatAnim, FocusedNode, ReactNode, ReactStyleState, StyleAnimationState,
};

/// Each frame: resolve interaction overrides, advance transitions, apply styles.
pub fn apply_interaction_styles(
    mut commands: Commands,
    time: Res<Time>,
    focused: Res<FocusedNode>,
    mut query: Query<(
        Entity,
        &Interaction,
        &ReactNode,
        &mut ReactStyleState,
        Option<&BackgroundColor>,
        Option<&BorderColor>,
        Option<&TextColor>,
    )>,
) {
    let dt = time.delta_secs();

    for (entity, interaction, react_node, mut state, bg, border, text_color) in &mut query {
        let is_focused =
            focused.entity == Some(entity) || focused.node_id == Some(react_node.node_id);

        let target = resolve_interaction_style(
            &state.base,
            state.hover.as_ref(),
            state.pressed.as_ref(),
            state.focused.as_ref(),
            *interaction,
            is_focused,
        );

        let transition = state.transition.clone();
        let displayed = advance_and_build_displayed(
            &mut state.anim,
            &transition,
            &target,
            bg.map(|c| c.0),
            border.and_then(|b| uniform_border_color(b)),
            text_color.map(|c| c.0),
            dt,
        );

        apply_resolved_style(&mut commands, entity, &displayed);
    }
}

fn uniform_border_color(border: &BorderColor) -> Option<Color> {
    let t = border.top;
    if colors_approx_eq(t, border.right)
        && colors_approx_eq(t, border.bottom)
        && colors_approx_eq(t, border.left)
    {
        Some(t)
    } else {
        Some(t)
    }
}

fn colors_approx_eq(a: Color, b: Color) -> bool {
    let a = a.to_srgba();
    let b = b.to_srgba();
    (a.red - b.red).abs() < 0.001
        && (a.green - b.green).abs() < 0.001
        && (a.blue - b.blue).abs() < 0.001
        && (a.alpha - b.alpha).abs() < 0.001
}

fn advance_and_build_displayed(
    anim: &mut StyleAnimationState,
    transition: &crate::react::style::StyleTransitions,
    target: &StyleProps,
    current_bg: Option<Color>,
    current_border: Option<Color>,
    current_text: Option<Color>,
    dt: f32,
) -> StyleProps {
    let mut displayed = target.clone();

    let target_bg = target.background_color.as_deref().and_then(parse_color);
    let bg = step_color_anim(
        &mut anim.background_color,
        target_bg,
        transition.duration_secs("backgroundColor"),
        current_bg,
        dt,
    );
    if let Some(c) = bg {
        displayed.background_color = Some(color_to_css(c));
    }

    let target_border = target.border_color.as_deref().and_then(parse_color);
    let border = step_color_anim(
        &mut anim.border_color,
        target_border,
        transition.duration_secs("borderColor"),
        current_border,
        dt,
    );
    if let Some(c) = border {
        displayed.border_color = Some(color_to_css(c));
    }

    let target_text = target.color.as_deref().and_then(parse_color);
    let text = step_color_anim(
        &mut anim.color,
        target_text,
        transition.duration_secs("color"),
        current_text,
        dt,
    );
    if let Some(c) = text {
        displayed.color = Some(color_to_css(c));
    }

    let target_opacity = style_opacity(target);
    let current_opacity = current_bg.map(|c| c.to_srgba().alpha);
    let opacity = step_float_anim(
        &mut anim.opacity,
        target_opacity,
        transition.duration_secs("opacity"),
        current_opacity,
        dt,
    );
    if let Some(o) = opacity {
        displayed.opacity = Some(crate::react::style::CssScalar(o.to_string()));
    }

    displayed
}

fn step_color_anim(
    track: &mut Option<ColorAnim>,
    target: Option<Color>,
    duration: Option<f32>,
    current: Option<Color>,
    dt: f32,
) -> Option<Color> {
    let Some(target) = target else {
        *track = None;
        return None;
    };

    let duration = duration.unwrap_or(0.0);

    match track.as_mut() {
        Some(anim) if colors_approx_eq(anim.to, target) => {
            anim.elapsed += dt;
            Some(anim.current())
        }
        Some(anim) => {
            let from = anim.current();
            if duration <= 0.0 {
                *track = None;
                return Some(target);
            }
            *track = Some(ColorAnim {
                from,
                to: target,
                elapsed: 0.0,
                duration,
            });
            Some(from)
        }
        None => {
            if duration <= 0.0 {
                return Some(target);
            }
            let from = current.unwrap_or(target);
            if colors_approx_eq(from, target) {
                return Some(target);
            }
            *track = Some(ColorAnim {
                from,
                to: target,
                elapsed: 0.0,
                duration,
            });
            Some(from)
        }
    }
}

fn step_float_anim(
    track: &mut Option<FloatAnim>,
    target: Option<f32>,
    duration: Option<f32>,
    current: Option<f32>,
    dt: f32,
) -> Option<f32> {
    let Some(target) = target else {
        *track = None;
        return None;
    };

    let duration = duration.unwrap_or(0.0);

    match track.as_mut() {
        Some(anim) if (anim.to - target).abs() < 0.0001 => {
            anim.elapsed += dt;
            Some(anim.current())
        }
        Some(anim) => {
            let from = anim.current();
            if duration <= 0.0 {
                *track = None;
                return Some(target);
            }
            *track = Some(FloatAnim {
                from,
                to: target,
                elapsed: 0.0,
                duration,
            });
            Some(from)
        }
        None => {
            if duration <= 0.0 {
                return Some(target);
            }
            let from = current.unwrap_or(target);
            if (from - target).abs() < 0.0001 {
                return Some(target);
            }
            *track = Some(FloatAnim {
                from,
                to: target,
                elapsed: 0.0,
                duration,
            });
            Some(from)
        }
    }
}

fn color_to_css(color: Color) -> String {
    let c = color.to_srgba();
    format!(
        "rgba({}, {}, {}, {})",
        (c.red * 255.0).round() as u8,
        (c.green * 255.0).round() as u8,
        (c.blue * 255.0).round() as u8,
        c.alpha
    )
}

fn apply_resolved_style(commands: &mut Commands, entity: Entity, style_props: &StyleProps) {
    commands.entity(entity).insert(json_to_style(style_props));

    match style_props.background_color.as_deref().and_then(parse_color) {
        Some(mut color) => {
            if let Some(opacity) = style_opacity(style_props) {
                color.set_alpha(opacity);
            }
            commands.entity(entity).insert(BackgroundColor(color));
        }
        None => {
            if let Some(opacity) = style_opacity(style_props) {
                let mut color = Color::WHITE;
                color.set_alpha(opacity);
                commands.entity(entity).insert(BackgroundColor(color));
            } else {
                commands.entity(entity).remove::<BackgroundColor>();
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

    match style_pointer_events(style_props) {
        Some(PointerEvents::None) => {
            commands.entity(entity).insert(Pickable::IGNORE);
            commands.entity(entity).insert(FocusPolicy::Pass);
        }
        Some(PointerEvents::Auto) => {
            commands.entity(entity).remove::<Pickable>();
            commands.entity(entity).insert(FocusPolicy::Block);
        }
        None => {}
    }

    if let Some(z) = style_props.z_index {
        commands.entity(entity).insert(ZIndex(z));
    }

    if let Some(ref color_str) = style_props.color
        && let Some(mut color) = parse_color(color_str)
    {
        if let Some(opacity) = style_opacity(style_props) {
            color.set_alpha(opacity);
        }
        commands.entity(entity).insert(TextColor(color));
    }
}

/// Insert or remove [`ReactStyleState`] from parsed style props (called from render).
pub fn sync_react_style_state(commands: &mut Commands, entity: Entity, style: Option<&StyleProps>) {
    match style {
        Some(props) if props.has_interaction_styles() => {
            commands.entity(entity).insert(ReactStyleState::from_props(props));
        }
        _ => {
            commands.entity(entity).remove::<ReactStyleState>();
        }
    }
}
