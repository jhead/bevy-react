use bevy::prelude::*;
use bevy::sprite::{BorderRect, TextureSlicer};
use bevy::text::{Justify, LineBreak, LineHeight, TextLayout};
use bevy::ui::widget::{NodeImageMode, TextShadow};
use bevy::ui::{
    AlignContent, AlignItems, AlignSelf, BackgroundGradient, BorderColor, BorderRadius, BoxShadow,
    ColorStop, Display, FlexDirection, FlexWrap, Gradient, GridAutoFlow, GridPlacement, GridTrack,
    JustifyContent, JustifyItems, JustifySelf, LinearGradient, Overflow, OverflowAxis,
    OverflowClipMargin, PositionType, RepeatedGridTrack, ShadowStyle, Val,
};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

/// A value that can be either a string or a number (for CSS-like length properties).
/// Numbers are treated as pixel values (`"Npx"`).
#[derive(Debug, Clone, Default)]
pub struct CssValue(pub String);

impl<'de> Deserialize<'de> for CssValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        match value {
            Value::String(s) => Ok(CssValue(s)),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(CssValue(format!("{}px", i)))
                } else if let Some(f) = n.as_f64() {
                    Ok(CssValue(format!("{}px", f)))
                } else {
                    Ok(CssValue(n.to_string()))
                }
            }
            _ => Ok(CssValue(String::new())),
        }
    }
}

/// A CSS scalar where bare numbers keep their numeric form (no implied `px`).
/// Used for `aspectRatio`, `lineHeight`, `opacity`, etc.
#[derive(Debug, Clone, Default)]
pub struct CssScalar(pub String);


impl<'de> Deserialize<'de> for CssScalar {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        match value {
            Value::String(s) => Ok(CssScalar(s)),
            Value::Number(n) => Ok(CssScalar(n.to_string())),
            _ => Ok(CssScalar(String::new())),
        }
    }
}

/// Props structure from React reconciler
#[derive(Debug, Default, Deserialize)]
pub struct NodeProps {
    #[serde(default)]
    pub style: Option<StyleProps>,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub src: Option<String>,
    /// Text content for <text> elements
    #[serde(default)]
    pub content: Option<String>,
}

/// Style properties from React (CSS-like)
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StyleProps {
    // Sizing
    pub width: Option<CssValue>,
    pub height: Option<CssValue>,
    pub min_width: Option<CssValue>,
    pub min_height: Option<CssValue>,
    pub max_width: Option<CssValue>,
    pub max_height: Option<CssValue>,
    pub aspect_ratio: Option<CssScalar>,

    // Flexbox
    pub flex_direction: Option<String>,
    pub flex_wrap: Option<String>,
    pub flex_grow: Option<f32>,
    pub flex_shrink: Option<f32>,
    pub flex_basis: Option<CssValue>,
    pub align_items: Option<String>,
    pub align_self: Option<String>,
    pub align_content: Option<String>,
    pub justify_content: Option<String>,
    pub justify_items: Option<String>,
    pub justify_self: Option<String>,

    // CSS Grid
    pub grid_template_columns: Option<String>,
    pub grid_template_rows: Option<String>,
    pub grid_auto_columns: Option<String>,
    pub grid_auto_rows: Option<String>,
    pub grid_auto_flow: Option<String>,
    pub grid_column: Option<String>,
    pub grid_row: Option<String>,
    pub grid_column_start: Option<CssScalar>,
    pub grid_column_end: Option<CssScalar>,
    pub grid_row_start: Option<CssScalar>,
    pub grid_row_end: Option<CssScalar>,

    // Spacing
    pub margin: Option<CssValue>,
    pub margin_top: Option<CssValue>,
    pub margin_right: Option<CssValue>,
    pub margin_bottom: Option<CssValue>,
    pub margin_left: Option<CssValue>,
    pub padding: Option<CssValue>,
    pub padding_top: Option<CssValue>,
    pub padding_right: Option<CssValue>,
    pub padding_bottom: Option<CssValue>,
    pub padding_left: Option<CssValue>,

    // Positioning
    pub position: Option<String>,
    pub top: Option<CssValue>,
    pub right: Option<CssValue>,
    pub bottom: Option<CssValue>,
    pub left: Option<CssValue>,

    // Border (borderWidth is the TS/CSS alias for uniform border width)
    pub border: Option<CssValue>,
    pub border_width: Option<CssValue>,
    pub border_top: Option<CssValue>,
    pub border_right: Option<CssValue>,
    pub border_bottom: Option<CssValue>,
    pub border_left: Option<CssValue>,
    pub border_radius: Option<CssValue>,
    pub border_top_left_radius: Option<CssValue>,
    pub border_top_right_radius: Option<CssValue>,
    pub border_bottom_right_radius: Option<CssValue>,
    pub border_bottom_left_radius: Option<CssValue>,

    // Gap
    pub gap: Option<CssValue>,
    pub row_gap: Option<CssValue>,
    pub column_gap: Option<CssValue>,

    // Display / overflow
    pub display: Option<String>,
    pub overflow: Option<String>,
    pub overflow_x: Option<String>,
    pub overflow_y: Option<String>,
    pub overflow_clip_margin: Option<String>,

    // Z-Index
    pub z_index: Option<i32>,

    // Colors
    pub background_color: Option<String>,
    pub border_color: Option<String>,
    pub border_top_color: Option<String>,
    pub border_right_color: Option<String>,
    pub border_bottom_color: Option<String>,
    pub border_left_color: Option<String>,

    // Visual effects
    pub opacity: Option<CssScalar>,
    pub box_shadow: Option<String>,
    pub background_image: Option<String>,
    /// Alias for CSS `background-image` linear-gradient / Bevy `BackgroundGradient`
    pub background_gradient: Option<String>,

    // Text styling
    pub color: Option<String>,
    pub font_size: Option<CssValue>,
    pub font_family: Option<String>,
    pub text_align: Option<String>,
    pub line_height: Option<CssScalar>,
    /// Soft wrap mode: `word` / `character` / `word-or-character` / `nowrap`.
    pub line_break: Option<String>,
    /// CSS-like `offset-x offset-y [blur] [color]` (blur ignored; Bevy has no text blur).
    pub text_shadow: Option<String>,

    // Image
    pub object_fit: Option<String>,
    /// Nine-slice border in px: `"16"` or `"top right bottom left"` / CSS 1–4 values.
    pub image_slice: Option<String>,
    pub tint: Option<String>,
    pub tint_color: Option<String>,
}

/// Parse props JSON into NodeProps
pub fn parse_props(props_json: &str) -> NodeProps {
    serde_json::from_str(props_json).unwrap_or_else(|e| {
        log::warn!("Failed to parse props JSON: {} - {}", props_json, e);
        NodeProps::default()
    })
}

/// Convert a CSS-like value string to Bevy's Val
/// Supports: "100px", "50%", "auto"
pub fn parse_val(value: &str) -> Val {
    let value = value.trim();

    if value == "auto" {
        return Val::Auto;
    }

    if let Some(px) = value.strip_suffix("px")
        && let Ok(n) = px.trim().parse::<f32>() {
            return Val::Px(n);
        }

    if let Some(pct) = value.strip_suffix("%")
        && let Ok(n) = pct.trim().parse::<f32>() {
            return Val::Percent(n);
        }

    if let Some(vw) = value.strip_suffix("vw")
        && let Ok(n) = vw.trim().parse::<f32>() {
            return Val::Vw(n);
        }

    if let Some(vh) = value.strip_suffix("vh")
        && let Ok(n) = vh.trim().parse::<f32>() {
            return Val::Vh(n);
        }

    // Try parsing as plain number (treat as px)
    if let Ok(n) = value.parse::<f32>() {
        return Val::Px(n);
    }

    log::warn!("Unknown Val format: '{}', defaulting to Auto", value);
    Val::Auto
}

/// Parse CSS margin/padding shorthand into a UiRect (1–4 values).
pub fn parse_ui_rect_shorthand(value: &str) -> UiRect {
    let parts: Vec<&str> = value.split_whitespace().collect();
    match parts.as_slice() {
        [all] => UiRect::all(parse_val(all)),
        [vertical, horizontal] => UiRect {
            top: parse_val(vertical),
            bottom: parse_val(vertical),
            left: parse_val(horizontal),
            right: parse_val(horizontal),
        },
        [top, horizontal, bottom] => UiRect {
            top: parse_val(top),
            right: parse_val(horizontal),
            bottom: parse_val(bottom),
            left: parse_val(horizontal),
        },
        [top, right, bottom, left] => UiRect {
            top: parse_val(top),
            right: parse_val(right),
            bottom: parse_val(bottom),
            left: parse_val(left),
        },
        _ => {
            log::warn!(
                "Invalid UiRect shorthand '{}', using first token for all sides",
                value
            );
            let first = parts.first().copied().unwrap_or("0");
            UiRect::all(parse_val(first))
        }
    }
}

/// Parse CSS border-radius shorthand into a BorderRadius (1–4 values).
pub fn parse_border_radius_shorthand(value: &str) -> BorderRadius {
    let parts: Vec<&str> = value.split_whitespace().collect();
    match parts.as_slice() {
        [all] => BorderRadius::all(parse_val(all)),
        [tl_br, tr_bl] => BorderRadius::new(
            parse_val(tl_br),
            parse_val(tr_bl),
            parse_val(tl_br),
            parse_val(tr_bl),
        ),
        [tl, tr_bl, br] => BorderRadius::new(
            parse_val(tl),
            parse_val(tr_bl),
            parse_val(br),
            parse_val(tr_bl),
        ),
        [tl, tr, br, bl] => BorderRadius::new(
            parse_val(tl),
            parse_val(tr),
            parse_val(br),
            parse_val(bl),
        ),
        _ => {
            log::warn!(
                "Invalid border-radius shorthand '{}', using first token",
                value
            );
            let first = parts.first().copied().unwrap_or("0");
            BorderRadius::all(parse_val(first))
        }
    }
}

/// Build BorderRadius from style props (shorthand + per-corner overrides).
pub fn style_to_border_radius(props: &StyleProps) -> Option<BorderRadius> {
    let has_any = props.border_radius.is_some()
        || props.border_top_left_radius.is_some()
        || props.border_top_right_radius.is_some()
        || props.border_bottom_right_radius.is_some()
        || props.border_bottom_left_radius.is_some();
    if !has_any {
        return None;
    }

    let mut radius = props
        .border_radius
        .as_ref()
        .map(|v| parse_border_radius_shorthand(&v.0))
        .unwrap_or(BorderRadius::ZERO);

    if let Some(ref v) = props.border_top_left_radius {
        radius.top_left = parse_val(&v.0);
    }
    if let Some(ref v) = props.border_top_right_radius {
        radius.top_right = parse_val(&v.0);
    }
    if let Some(ref v) = props.border_bottom_right_radius {
        radius.bottom_right = parse_val(&v.0);
    }
    if let Some(ref v) = props.border_bottom_left_radius {
        radius.bottom_left = parse_val(&v.0);
    }
    Some(radius)
}

/// Build per-side BorderColor from style props.
pub fn style_to_border_color(props: &StyleProps) -> Option<BorderColor> {
    let has_any = props.border_color.is_some()
        || props.border_top_color.is_some()
        || props.border_right_color.is_some()
        || props.border_bottom_color.is_some()
        || props.border_left_color.is_some();
    if !has_any {
        return None;
    }

    let mut color = BorderColor::DEFAULT;
    if let Some(ref c) = props.border_color
        && let Some(parsed) = parse_color(c) {
            color = BorderColor::all(parsed);
        }
    if let Some(ref c) = props.border_top_color
        && let Some(parsed) = parse_color(c) {
            color.top = parsed;
        }
    if let Some(ref c) = props.border_right_color
        && let Some(parsed) = parse_color(c) {
            color.right = parsed;
        }
    if let Some(ref c) = props.border_bottom_color
        && let Some(parsed) = parse_color(c) {
            color.bottom = parsed;
        }
    if let Some(ref c) = props.border_left_color
        && let Some(parsed) = parse_color(c) {
            color.left = parsed;
        }
    Some(color)
}

/// Parse opacity (0–1, or percentage string).
pub fn parse_opacity(value: &str) -> Option<f32> {
    let value = value.trim();
    if let Some(pct) = value.strip_suffix('%') {
        return pct.trim().parse::<f32>().ok().map(|n| (n / 100.0).clamp(0.0, 1.0));
    }
    value.parse::<f32>().ok().map(|n| n.clamp(0.0, 1.0))
}

pub fn style_opacity(props: &StyleProps) -> Option<f32> {
    props.opacity.as_ref().and_then(|v| parse_opacity(&v.0))
}

/// Parse CSS `box-shadow` into Bevy BoxShadow.
/// Supports: `offset-x offset-y [blur] [spread] [color]` (comma-separated layers).
pub fn parse_box_shadow(value: &str) -> Option<BoxShadow> {
    let layers: Vec<ShadowStyle> = split_css_list(value)
        .into_iter()
        .filter_map(|layer| parse_box_shadow_layer(layer.trim()))
        .collect();
    if layers.is_empty() {
        None
    } else {
        Some(BoxShadow(layers))
    }
}

fn parse_box_shadow_layer(layer: &str) -> Option<ShadowStyle> {
    if layer.is_empty() || layer == "none" {
        return None;
    }

    let tokens: Vec<&str> = layer.split_whitespace().collect();
    let mut lengths: Vec<Val> = Vec::new();
    let mut color = Color::srgba(0.0, 0.0, 0.0, 0.5);

    for token in tokens {
        let lower = token.to_lowercase();
        if lower.starts_with('#')
            || lower.starts_with("rgb")
            || lower.starts_with("hsl")
            || named_color(&lower).is_some()
        {
            if let Some(c) = parse_color(token) {
                color = c;
            }
        } else {
            lengths.push(parse_val(token));
        }
    }

    if lengths.is_empty() {
        return None;
    }

    Some(ShadowStyle {
        color,
        x_offset: lengths.first().copied().unwrap_or(Val::Px(0.0)),
        y_offset: lengths.get(1).copied().unwrap_or(Val::Px(0.0)),
        blur_radius: lengths.get(2).copied().unwrap_or(Val::Px(0.0)),
        spread_radius: lengths.get(3).copied().unwrap_or(Val::Px(0.0)),
    })
}

pub fn style_to_box_shadow(props: &StyleProps) -> Option<BoxShadow> {
    props.box_shadow.as_deref().and_then(parse_box_shadow)
}

/// Parse a CSS `linear-gradient(...)` into Bevy BackgroundGradient.
pub fn parse_background_gradient(value: &str) -> Option<BackgroundGradient> {
    let value = value.trim();
    let lower = value.to_lowercase();
    if !lower.starts_with("linear-gradient(") {
        return None;
    }
    let inner = value
        .trim_start_matches(|c: char| c != '(')
        .trim_start_matches('(')
        .trim_end_matches(')')
        .trim();

    // Split on commas not inside parentheses
    let parts = split_css_list(inner);
    if parts.is_empty() {
        return None;
    }

    let mut angle = LinearGradient::TO_BOTTOM;
    let mut stop_start = 0;

    let first = parts[0].trim().to_lowercase();
    if first.starts_with("to ") {
        angle = match first.as_str() {
            "to top" => LinearGradient::TO_TOP,
            "to top right" | "to right top" => LinearGradient::TO_TOP_RIGHT,
            "to right" => LinearGradient::TO_RIGHT,
            "to bottom right" | "to right bottom" => LinearGradient::TO_BOTTOM_RIGHT,
            "to bottom" => LinearGradient::TO_BOTTOM,
            "to bottom left" | "to left bottom" => LinearGradient::TO_BOTTOM_LEFT,
            "to left" => LinearGradient::TO_LEFT,
            "to top left" | "to left top" => LinearGradient::TO_TOP_LEFT,
            _ => LinearGradient::TO_BOTTOM,
        };
        stop_start = 1;
    } else if let Some(deg) = first.strip_suffix("deg")
        && let Ok(degrees) = deg.trim().parse::<f32>() {
            // CSS: 0deg = to top; Bevy: 0 = to top, increasing clockwise — same convention
            angle = degrees.to_radians();
            stop_start = 1;
        }

    let mut stops = Vec::new();
    for part in &parts[stop_start..] {
        let tokens: Vec<&str> = part.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }
        let color = parse_color(tokens[0])?;
        let point = if tokens.len() >= 2 {
            parse_val(tokens[1])
        } else {
            Val::Auto
        };
        stops.push(ColorStop::new(color, point));
    }

    if stops.len() < 2 {
        return None;
    }

    Some(BackgroundGradient(vec![Gradient::Linear(LinearGradient::new(
        angle, stops,
    ))]))
}

pub fn style_to_background_gradient(props: &StyleProps) -> Option<BackgroundGradient> {
    props
        .background_gradient
        .as_deref()
        .or(props.background_image.as_deref())
        .and_then(parse_background_gradient)
}

/// Parse CSS text-align into Bevy Justify (JustifyText).
pub fn parse_text_align(value: &str) -> Option<Justify> {
    match value.trim().to_lowercase().as_str() {
        "left" | "start" => Some(Justify::Left),
        "right" | "end" => Some(Justify::Right),
        "center" => Some(Justify::Center),
        "justify" => Some(Justify::Justified),
        _ => None,
    }
}

pub fn style_text_align(props: &StyleProps) -> Option<Justify> {
    props.text_align.as_deref().and_then(parse_text_align)
}

/// Parse line-break / white-space keywords into Bevy [`LineBreak`].
pub fn parse_line_break(value: &str) -> Option<LineBreak> {
    match value.trim().to_lowercase().as_str() {
        "word" | "word-boundary" | "wordboundary" | "normal" => Some(LineBreak::WordBoundary),
        "character" | "any-character" | "anycharacter" | "anywhere" | "break-all" => {
            Some(LineBreak::AnyCharacter)
        }
        "word-or-character" | "wordorcharacter" | "break-word" => {
            Some(LineBreak::WordOrCharacter)
        }
        "nowrap" | "no-wrap" | "none" => Some(LineBreak::NoWrap),
        _ => None,
    }
}

pub fn style_line_break(props: &StyleProps) -> Option<LineBreak> {
    props.line_break.as_deref().and_then(parse_line_break)
}

/// Combine `textAlign` + `lineBreak` into a single [`TextLayout`] when either is set.
pub fn style_text_layout(props: &StyleProps) -> Option<TextLayout> {
    let justify = style_text_align(props);
    let linebreak = style_line_break(props);
    match (justify, linebreak) {
        (None, None) => None,
        (Some(j), Some(lb)) => Some(TextLayout::new(j, lb)),
        (Some(j), None) => Some(TextLayout::new_with_justify(j)),
        (None, Some(lb)) => Some(TextLayout::new_with_linebreak(lb)),
    }
}

/// Parse CSS-like `text-shadow`: `offset-x offset-y [blur] [color]`.
/// Blur is accepted and ignored (Bevy [`TextShadow`] has no blur radius).
pub fn parse_text_shadow(value: &str) -> Option<TextShadow> {
    let value = value.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("none") {
        return None;
    }

    let tokens: Vec<&str> = value.split_whitespace().collect();
    let mut lengths: Vec<f32> = Vec::new();
    let mut color = Color::linear_rgba(0.0, 0.0, 0.0, 0.75);

    for token in tokens {
        let lower = token.to_lowercase();
        if lower.starts_with('#')
            || lower.starts_with("rgb")
            || lower.starts_with("hsl")
            || named_color(&lower).is_some()
        {
            if let Some(c) = parse_color(token) {
                color = c;
            }
        } else if let Some(px) = length_to_px(token) {
            lengths.push(px);
        }
    }

    if lengths.is_empty() {
        return None;
    }

    Some(TextShadow {
        offset: Vec2::new(
            lengths.first().copied().unwrap_or(0.0),
            lengths.get(1).copied().unwrap_or(0.0),
        ),
        color,
    })
}

pub fn style_text_shadow(props: &StyleProps) -> Option<TextShadow> {
    props.text_shadow.as_deref().and_then(parse_text_shadow)
}

fn length_to_px(token: &str) -> Option<f32> {
    let token = token.trim();
    if let Some(px) = token.strip_suffix("px") {
        return px.trim().parse().ok();
    }
    if token.ends_with('%') || token.eq_ignore_ascii_case("auto") {
        return None;
    }
    token.parse().ok()
}

/// Parse line-height: unitless → RelativeToFont, px/% → Px / RelativeToFont.
pub fn parse_line_height(value: &str) -> Option<LineHeight> {
    let value = value.trim();
    if let Some(px) = value.strip_suffix("px") {
        return px.trim().parse::<f32>().ok().map(LineHeight::Px);
    }
    if let Some(pct) = value.strip_suffix('%') {
        return pct
            .trim()
            .parse::<f32>()
            .ok()
            .map(|n| LineHeight::RelativeToFont(n / 100.0));
    }
    value
        .parse::<f32>()
        .ok()
        .map(LineHeight::RelativeToFont)
}

pub fn style_line_height(props: &StyleProps) -> Option<LineHeight> {
    props.line_height.as_ref().and_then(|v| parse_line_height(&v.0))
}

/// Asset path for fontFamily when it looks like a path; `None` for generic families.
pub fn parse_font_family(value: &str) -> Option<String> {
    let value = value.trim().trim_matches('"').trim_matches('\'');
    if value.is_empty() {
        return None;
    }
    let lower = value.to_lowercase();
    match lower.as_str() {
        "serif" | "sans-serif" | "monospace" | "cursive" | "fantasy" | "system-ui" | "inherit"
        | "initial" | "unset" => None,
        _ if value.contains('/') || value.contains('.') => Some(value.to_string()),
        _ => Some(value.to_string()),
    }
}

pub fn style_font_family(props: &StyleProps) -> Option<String> {
    props.font_family.as_deref().and_then(parse_font_family)
}

/// Map CSS object-fit to Bevy NodeImageMode.
pub fn parse_object_fit(value: &str) -> NodeImageMode {
    match value.trim().to_lowercase().as_str() {
        "fill" | "stretch" => NodeImageMode::Stretch,
        "none" | "contain" | "cover" | "scale-down" | "auto" => NodeImageMode::Auto,
        _ => NodeImageMode::Auto,
    }
}

pub fn style_object_fit(props: &StyleProps) -> Option<NodeImageMode> {
    props.object_fit.as_deref().map(parse_object_fit)
}

/// Parse nine-slice border insets: `"16"` or up to four px lengths (CSS TRBL order).
pub fn parse_image_slice(value: &str) -> Option<NodeImageMode> {
    let parts: Vec<f32> = value
        .split_whitespace()
        .filter_map(length_to_px)
        .collect();
    let border = match parts.as_slice() {
        [all] => BorderRect::all(*all),
        [v, h] => BorderRect {
            top: *v,
            bottom: *v,
            left: *h,
            right: *h,
        },
        [t, h, b] => BorderRect {
            top: *t,
            right: *h,
            bottom: *b,
            left: *h,
        },
        [t, r, b, l] => BorderRect {
            top: *t,
            right: *r,
            bottom: *b,
            left: *l,
        },
        _ => return None,
    };
    Some(NodeImageMode::Sliced(TextureSlicer {
        border,
        ..Default::default()
    }))
}

/// Resolve image mode: `imageSlice` wins when set, otherwise `objectFit`.
pub fn style_image_mode(props: &StyleProps) -> Option<NodeImageMode> {
    if let Some(slice) = props.image_slice.as_deref().and_then(parse_image_slice) {
        return Some(slice);
    }
    style_object_fit(props)
}

pub fn style_tint(props: &StyleProps) -> Option<Color> {
    props
        .tint
        .as_deref()
        .or(props.tint_color.as_deref())
        .and_then(parse_color)
}

/// Parse aspect-ratio: `1.5`, `16/9`, `16 / 9`.
pub fn parse_aspect_ratio(value: &str) -> Option<f32> {
    let value = value.trim();
    if let Some((w, h)) = value.split_once('/') {
        let w: f32 = w.trim().parse().ok()?;
        let h: f32 = h.trim().parse().ok()?;
        if h == 0.0 {
            return None;
        }
        return Some(w / h);
    }
    value.parse().ok()
}

fn parse_flex_direction(value: &str) -> FlexDirection {
    match value.to_lowercase().as_str() {
        "row" => FlexDirection::Row,
        "row-reverse" | "rowreverse" => FlexDirection::RowReverse,
        "column" | "col" => FlexDirection::Column,
        "column-reverse" | "columnreverse" => FlexDirection::ColumnReverse,
        _ => FlexDirection::default(),
    }
}

fn parse_align_items(value: &str) -> AlignItems {
    match value.to_lowercase().as_str() {
        "start" | "flex-start" | "flexstart" => AlignItems::FlexStart,
        "end" | "flex-end" | "flexend" => AlignItems::FlexEnd,
        "center" => AlignItems::Center,
        "baseline" => AlignItems::Baseline,
        "stretch" => AlignItems::Stretch,
        _ => AlignItems::default(),
    }
}

fn parse_justify_content(value: &str) -> JustifyContent {
    match value.to_lowercase().as_str() {
        "start" | "flex-start" | "flexstart" => JustifyContent::FlexStart,
        "end" | "flex-end" | "flexend" => JustifyContent::FlexEnd,
        "center" => JustifyContent::Center,
        "space-between" | "spacebetween" => JustifyContent::SpaceBetween,
        "space-around" | "spacearound" => JustifyContent::SpaceAround,
        "space-evenly" | "spaceevenly" => JustifyContent::SpaceEvenly,
        _ => JustifyContent::default(),
    }
}

fn parse_position_type(value: &str) -> PositionType {
    match value.to_lowercase().as_str() {
        "relative" => PositionType::Relative,
        "absolute" => PositionType::Absolute,
        _ => PositionType::default(),
    }
}

fn parse_flex_wrap(value: &str) -> FlexWrap {
    match value.to_lowercase().as_str() {
        "nowrap" | "no-wrap" => FlexWrap::NoWrap,
        "wrap" => FlexWrap::Wrap,
        "wrap-reverse" | "wrapreverse" => FlexWrap::WrapReverse,
        _ => FlexWrap::default(),
    }
}

fn parse_align_self(value: &str) -> AlignSelf {
    match value.to_lowercase().as_str() {
        "auto" => AlignSelf::Auto,
        "start" | "flex-start" | "flexstart" => AlignSelf::FlexStart,
        "end" | "flex-end" | "flexend" => AlignSelf::FlexEnd,
        "center" => AlignSelf::Center,
        "baseline" => AlignSelf::Baseline,
        "stretch" => AlignSelf::Stretch,
        _ => AlignSelf::default(),
    }
}

fn parse_align_content(value: &str) -> AlignContent {
    match value.to_lowercase().as_str() {
        "start" | "flex-start" | "flexstart" => AlignContent::FlexStart,
        "end" | "flex-end" | "flexend" => AlignContent::FlexEnd,
        "center" => AlignContent::Center,
        "stretch" => AlignContent::Stretch,
        "space-between" | "spacebetween" => AlignContent::SpaceBetween,
        "space-around" | "spacearound" => AlignContent::SpaceAround,
        "space-evenly" | "spaceevenly" => AlignContent::SpaceEvenly,
        _ => AlignContent::default(),
    }
}

fn parse_justify_items(value: &str) -> JustifyItems {
    match value.to_lowercase().as_str() {
        "start" | "flex-start" | "flexstart" => JustifyItems::Start,
        "end" | "flex-end" | "flexend" => JustifyItems::End,
        "center" => JustifyItems::Center,
        "baseline" => JustifyItems::Baseline,
        "stretch" => JustifyItems::Stretch,
        _ => JustifyItems::default(),
    }
}

fn parse_justify_self(value: &str) -> JustifySelf {
    match value.to_lowercase().as_str() {
        "auto" => JustifySelf::Auto,
        "start" | "flex-start" | "flexstart" => JustifySelf::Start,
        "end" | "flex-end" | "flexend" => JustifySelf::End,
        "center" => JustifySelf::Center,
        "baseline" => JustifySelf::Baseline,
        "stretch" => JustifySelf::Stretch,
        _ => JustifySelf::default(),
    }
}

fn parse_display(value: &str) -> Display {
    match value.to_lowercase().as_str() {
        "flex" => Display::Flex,
        "none" => Display::None,
        "grid" => Display::Grid,
        "block" => Display::Block,
        _ => Display::default(),
    }
}

fn parse_overflow_axis(value: &str) -> OverflowAxis {
    match value.to_lowercase().as_str() {
        "visible" => OverflowAxis::Visible,
        "clip" | "hidden" => OverflowAxis::Clip,
        "scroll" => OverflowAxis::Scroll,
        _ => OverflowAxis::Visible,
    }
}

/// Parse overflow (applies to both axes)
fn parse_overflow(value: &str) -> Overflow {
    let axis = parse_overflow_axis(value);
    Overflow { x: axis, y: axis }
}

fn parse_overflow_clip_margin(value: &str) -> OverflowClipMargin {
    let parts: Vec<&str> = value.split_whitespace().collect();
    let (box_token, margin_token) = match parts.as_slice() {
        [b] => (*b, None),
        [b, m] => (*b, Some(*m)),
        _ => (value.trim(), None),
    };

    let mut clip = match box_token.to_lowercase().as_str() {
        "content-box" | "contentbox" => OverflowClipMargin::content_box(),
        "padding-box" | "paddingbox" => OverflowClipMargin::padding_box(),
        "border-box" | "borderbox" => OverflowClipMargin::border_box(),
        _ => {
            // Bare length → padding-box with margin
            if let Val::Px(px) = parse_val(box_token) {
                return OverflowClipMargin::padding_box().with_margin(px);
            }
            OverflowClipMargin::DEFAULT
        }
    };

    if let Some(m) = margin_token
        && let Val::Px(px) = parse_val(m) {
            clip = clip.with_margin(px);
        }
    clip
}

fn parse_grid_auto_flow(value: &str) -> GridAutoFlow {
    match value.to_lowercase().as_str() {
        "row" => GridAutoFlow::Row,
        "column" | "col" => GridAutoFlow::Column,
        "row dense" | "rowdense" | "dense" => GridAutoFlow::RowDense,
        "column dense" | "columndense" => GridAutoFlow::ColumnDense,
        _ => GridAutoFlow::default(),
    }
}

/// Parse a single grid track sizing function into a RepeatedGridTrack (count 1).
fn parse_grid_track(token: &str) -> Option<RepeatedGridTrack> {
    let token = token.trim().to_lowercase();
    if token.is_empty() {
        return None;
    }

    if token == "auto" {
        return Some(GridTrack::auto());
    }
    if token == "min-content" || token == "mincontent" {
        return Some(GridTrack::min_content());
    }
    if token == "max-content" || token == "maxcontent" {
        return Some(GridTrack::max_content());
    }

    if let Some(fr) = token.strip_suffix("fr")
        && let Ok(n) = fr.trim().parse::<f32>() {
            return Some(GridTrack::flex(n));
        }

    if let Some(px) = token.strip_suffix("px")
        && let Ok(n) = px.trim().parse::<f32>() {
            return Some(GridTrack::px(n));
        }

    if let Some(pct) = token.strip_suffix('%')
        && let Ok(n) = pct.trim().parse::<f32>() {
            return Some(GridTrack::percent(n));
        }

    if let Some(inner) = token
        .strip_prefix("fit-content(")
        .and_then(|s| s.strip_suffix(')'))
    {
        if let Some(px) = inner.strip_suffix("px")
            && let Ok(n) = px.trim().parse::<f32>() {
                return Some(GridTrack::fit_content_px(n));
            }
        if let Some(pct) = inner.strip_suffix('%')
            && let Ok(n) = pct.trim().parse::<f32>() {
                return Some(GridTrack::fit_content_percent(n));
            }
    }

    // Plain number → px
    if let Ok(n) = token.parse::<f32>() {
        return Some(GridTrack::px(n));
    }

    log::warn!("Unknown grid track '{}', defaulting to auto", token);
    Some(GridTrack::auto())
}

/// Parse `grid-template-columns` / `grid-template-rows` track lists.
/// Supports space-separated tracks and `repeat(N, track)`.
pub fn parse_grid_template(value: &str) -> Vec<RepeatedGridTrack> {
    let mut tracks = Vec::new();
    let mut rest = value.trim();

    while !rest.is_empty() {
        rest = rest.trim_start();
        if rest.is_empty() {
            break;
        }

        if let Some(after_repeat) = rest.strip_prefix("repeat(")
            && let Some(end) = find_closing_paren(after_repeat) {
                let inner = &after_repeat[..end];
                if let Some((count_str, track_str)) = inner.split_once(',') {
                    let count_str = count_str.trim().to_lowercase();
                    let track_str = track_str.trim();
                    let repetition = match count_str.as_str() {
                        "auto-fill" | "autofill" => {
                            // Auto-fill/fit only support fixed tracks via RepeatedGridTrack helpers;
                            // fall back to a single auto track when unsupported.
                            if let Some(t) = parse_grid_track(track_str) {
                                tracks.push(t);
                            }
                            rest = after_repeat[end + 1..].trim_start();
                            continue;
                        }
                        "auto-fit" | "autofit" => {
                            if let Some(t) = parse_grid_track(track_str) {
                                tracks.push(t);
                            }
                            rest = after_repeat[end + 1..].trim_start();
                            continue;
                        }
                        _ => count_str.parse::<u16>().unwrap_or(1),
                    };

                    if let Some(base) = parse_grid_track(track_str) {
                        // Expand integer repeats into N identical tracks
                        for _ in 0..repetition {
                            tracks.push(base.clone());
                        }
                    }
                }
                rest = after_repeat[end + 1..].trim_start();
                continue;
            }

        // Take next whitespace-delimited token (or minmax/fit-content call)
        let (token, remaining) = next_css_token(rest);
        if let Some(t) = parse_grid_track(token) {
            tracks.push(t);
        }
        rest = remaining;
    }

    tracks
}

fn parse_grid_auto_tracks(value: &str) -> Vec<GridTrack> {
    value
        .split_whitespace()
        .filter_map(|token| {
            // parse_grid_track returns RepeatedGridTrack; extract via constructors again
            let token = token.trim().to_lowercase();
            if token == "auto" {
                return Some(GridTrack::auto());
            }
            if token == "min-content" || token == "mincontent" {
                return Some(GridTrack::min_content());
            }
            if token == "max-content" || token == "maxcontent" {
                return Some(GridTrack::max_content());
            }
            if let Some(fr) = token.strip_suffix("fr")
                && let Ok(n) = fr.trim().parse::<f32>() {
                    return Some(GridTrack::flex(n));
                }
            if let Some(px) = token.strip_suffix("px")
                && let Ok(n) = px.trim().parse::<f32>() {
                    return Some(GridTrack::px(n));
                }
            if let Some(pct) = token.strip_suffix('%')
                && let Ok(n) = pct.trim().parse::<f32>() {
                    return Some(GridTrack::percent(n));
                }
            token.parse::<f32>().ok().map(GridTrack::px)
        })
        .collect()
}

/// Parse CSS grid-row / grid-column placement.
pub fn parse_grid_placement(value: &str) -> GridPlacement {
    let value = value.trim().to_lowercase();
    if value == "auto" {
        return GridPlacement::auto();
    }

    let parts: Vec<&str> = value.split('/').map(|s| s.trim()).collect();
    match parts.as_slice() {
        [start] => {
            if let Some(span) = start.strip_prefix("span ")
                && let Ok(n) = span.trim().parse::<u16>() {
                    return GridPlacement::span(n.max(1));
                }
            if let Ok(n) = start.parse::<i16>()
                && n != 0 {
                    return GridPlacement::start(n);
                }
            GridPlacement::auto()
        }
        [start, end] => {
            let start = start.trim();
            let end = end.trim();
            if let Some(span) = end.strip_prefix("span ") {
                let span_n = span.trim().parse::<u16>().unwrap_or(1).max(1);
                if let Ok(s) = start.parse::<i16>()
                    && s != 0 {
                        return GridPlacement::start_span(s, span_n);
                    }
                return GridPlacement::span(span_n);
            }
            if start == "span" || start.starts_with("span ") {
                // unusual; treat as span only
                if let Some(span) = start.strip_prefix("span ")
                    && let Ok(n) = span.trim().parse::<u16>() {
                        return GridPlacement::span(n.max(1));
                    }
            }
            let s = start.parse::<i16>().ok();
            let e = end.parse::<i16>().ok();
            match (s, e) {
                (Some(s), Some(e)) if s != 0 && e != 0 => GridPlacement::start_end(s, e),
                (Some(s), _) if s != 0 => GridPlacement::start(s),
                (_, Some(e)) if e != 0 => GridPlacement::end(e),
                _ => GridPlacement::auto(),
            }
        }
        _ => GridPlacement::auto(),
    }
}

fn build_grid_placement(
    shorthand: Option<&String>,
    start: Option<&CssScalar>,
    end: Option<&CssScalar>,
) -> Option<GridPlacement> {
    if let Some(s) = shorthand {
        return Some(parse_grid_placement(s));
    }
    match (start, end) {
        (Some(s), Some(e)) => {
            let start_n = s.0.trim().parse::<i16>().ok()?;
            let end_n = e.0.trim().parse::<i16>().ok()?;
            if start_n == 0 || end_n == 0 {
                return None;
            }
            Some(GridPlacement::start_end(start_n, end_n))
        }
        (Some(s), None) => {
            let start_n = s.0.trim().parse::<i16>().ok()?;
            if start_n == 0 {
                return None;
            }
            Some(GridPlacement::start(start_n))
        }
        (None, Some(e)) => {
            let end_n = e.0.trim().parse::<i16>().ok()?;
            if end_n == 0 {
                return None;
            }
            Some(GridPlacement::end(end_n))
        }
        (None, None) => None,
    }
}

fn find_closing_paren(s: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                if depth == 0 {
                    return Some(i);
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    None
}

fn next_css_token(s: &str) -> (&str, &str) {
    let s = s.trim_start();
    if s.is_empty() {
        return ("", "");
    }
    // Function call token
    if let Some(paren) = s.find('(') {
        let name = &s[..paren];
        if !name.contains(char::is_whitespace)
            && let Some(end) = find_closing_paren(&s[paren + 1..]) {
                let end_abs = paren + 1 + end + 1;
                return (&s[..end_abs], s[end_abs..].trim_start());
            }
    }
    if let Some(ws) = s.find(char::is_whitespace) {
        (&s[..ws], s[ws..].trim_start())
    } else {
        (s, "")
    }
}

fn split_css_list(s: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut depth = 0usize;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                parts.push(s[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    let last = s[start..].trim();
    if !last.is_empty() {
        parts.push(last);
    }
    parts
}

/// Parse a CSS color string to Bevy Color.
/// Supports named colors, hex, rgb/rgba (legacy + modern), hsl/hsla.
pub fn parse_color(value: &str) -> Option<Color> {
    let value = value.trim().to_lowercase();

    if let Some(c) = named_color(&value) {
        return Some(c);
    }

    if let Some(hex) = value.strip_prefix('#') {
        return parse_hex_color(hex);
    }

    if value.starts_with("hsl") {
        return parse_hsl_color(&value);
    }

    if value.starts_with("rgb") {
        return parse_rgb_color(&value);
    }

    log::warn!("Unknown color format: '{}'", value);
    None
}

fn named_color(value: &str) -> Option<Color> {
    // CSS Level 1–3 named colors (subset of full table; includes common aliases)
    let (r, g, b) = match value {
        "transparent" => return Some(Color::NONE),
        "black" => (0, 0, 0),
        "silver" => (192, 192, 192),
        "gray" | "grey" => (128, 128, 128),
        "white" => (255, 255, 255),
        "maroon" => (128, 0, 0),
        "red" => (255, 0, 0),
        "purple" => (128, 0, 128),
        "fuchsia" | "magenta" => (255, 0, 255),
        "green" => (0, 128, 0),
        "lime" => (0, 255, 0),
        "olive" => (128, 128, 0),
        "yellow" => (255, 255, 0),
        "navy" => (0, 0, 128),
        "blue" => (0, 0, 255),
        "teal" => (0, 128, 128),
        "aqua" | "cyan" => (0, 255, 255),
        "orange" => (255, 165, 0),
        "aliceblue" => (240, 248, 255),
        "antiquewhite" => (250, 235, 215),
        "aquamarine" => (127, 255, 212),
        "azure" => (240, 255, 255),
        "beige" => (245, 245, 220),
        "bisque" => (255, 228, 196),
        "blanchedalmond" => (255, 235, 205),
        "blueviolet" => (138, 43, 226),
        "brown" => (165, 42, 42),
        "burlywood" => (222, 184, 135),
        "cadetblue" => (95, 158, 160),
        "chartreuse" => (127, 255, 0),
        "chocolate" => (210, 105, 30),
        "coral" => (255, 127, 80),
        "cornflowerblue" => (100, 149, 237),
        "cornsilk" => (255, 248, 220),
        "crimson" => (220, 20, 60),
        "darkblue" => (0, 0, 139),
        "darkcyan" => (0, 139, 139),
        "darkgoldenrod" => (184, 134, 11),
        "darkgray" | "darkgrey" => (169, 169, 169),
        "darkgreen" => (0, 100, 0),
        "darkkhaki" => (189, 183, 107),
        "darkmagenta" => (139, 0, 139),
        "darkolivegreen" => (85, 107, 47),
        "darkorange" => (255, 140, 0),
        "darkorchid" => (153, 50, 204),
        "darkred" => (139, 0, 0),
        "darksalmon" => (233, 150, 122),
        "darkseagreen" => (143, 188, 143),
        "darkslateblue" => (72, 61, 139),
        "darkslategray" | "darkslategrey" => (47, 79, 79),
        "darkturquoise" => (0, 206, 209),
        "darkviolet" => (148, 0, 211),
        "deeppink" => (255, 20, 147),
        "deepskyblue" => (0, 191, 255),
        "dimgray" | "dimgrey" => (105, 105, 105),
        "dodgerblue" => (30, 144, 255),
        "firebrick" => (178, 34, 34),
        "floralwhite" => (255, 250, 240),
        "forestgreen" => (34, 139, 34),
        "gainsboro" => (220, 220, 220),
        "ghostwhite" => (248, 248, 255),
        "gold" => (255, 215, 0),
        "goldenrod" => (218, 165, 32),
        "greenyellow" => (173, 255, 47),
        "honeydew" => (240, 255, 240),
        "hotpink" => (255, 105, 180),
        "indianred" => (205, 92, 92),
        "indigo" => (75, 0, 130),
        "ivory" => (255, 255, 240),
        "khaki" => (240, 230, 140),
        "lavender" => (230, 230, 250),
        "lavenderblush" => (255, 240, 245),
        "lawngreen" => (124, 252, 0),
        "lemonchiffon" => (255, 250, 205),
        "lightblue" => (173, 216, 230),
        "lightcoral" => (240, 128, 128),
        "lightcyan" => (224, 255, 255),
        "lightgoldenrodyellow" => (250, 250, 210),
        "lightgray" | "lightgrey" => (211, 211, 211),
        "lightgreen" => (144, 238, 144),
        "lightpink" => (255, 182, 193),
        "lightsalmon" => (255, 160, 122),
        "lightseagreen" => (32, 178, 170),
        "lightskyblue" => (135, 206, 250),
        "lightslategray" | "lightslategrey" => (119, 136, 153),
        "lightsteelblue" => (176, 196, 222),
        "lightyellow" => (255, 255, 224),
        "limegreen" => (50, 205, 50),
        "linen" => (250, 240, 230),
        "mediumaquamarine" => (102, 205, 170),
        "mediumblue" => (0, 0, 205),
        "mediumorchid" => (186, 85, 211),
        "mediumpurple" => (147, 112, 219),
        "mediumseagreen" => (60, 179, 113),
        "mediumslateblue" => (123, 104, 238),
        "mediumspringgreen" => (0, 250, 154),
        "mediumturquoise" => (72, 209, 204),
        "mediumvioletred" => (199, 21, 133),
        "midnightblue" => (25, 25, 112),
        "mintcream" => (245, 255, 250),
        "mistyrose" => (255, 228, 225),
        "moccasin" => (255, 228, 181),
        "navajowhite" => (255, 222, 173),
        "oldlace" => (253, 245, 230),
        "olivedrab" => (107, 142, 35),
        "orangered" => (255, 69, 0),
        "orchid" => (218, 112, 214),
        "palegoldenrod" => (238, 232, 170),
        "palegreen" => (152, 251, 152),
        "paleturquoise" => (175, 238, 238),
        "palevioletred" => (219, 112, 147),
        "papayawhip" => (255, 239, 213),
        "peachpuff" => (255, 218, 185),
        "peru" => (205, 133, 63),
        "pink" => (255, 192, 203),
        "plum" => (221, 160, 221),
        "powderblue" => (176, 224, 230),
        "rebeccapurple" => (102, 51, 153),
        "rosybrown" => (188, 143, 143),
        "royalblue" => (65, 105, 225),
        "saddlebrown" => (139, 69, 19),
        "salmon" => (250, 128, 114),
        "sandybrown" => (244, 164, 96),
        "seagreen" => (46, 139, 87),
        "seashell" => (255, 245, 238),
        "sienna" => (160, 82, 45),
        "skyblue" => (135, 206, 235),
        "slateblue" => (106, 90, 205),
        "slategray" | "slategrey" => (112, 128, 144),
        "snow" => (255, 250, 250),
        "springgreen" => (0, 255, 127),
        "steelblue" => (70, 130, 180),
        "tan" => (210, 180, 140),
        "thistle" => (216, 191, 216),
        "tomato" => (255, 99, 71),
        "turquoise" => (64, 224, 208),
        "violet" => (238, 130, 238),
        "wheat" => (245, 222, 179),
        "whitesmoke" => (245, 245, 245),
        "yellowgreen" => (154, 205, 50),
        _ => return None,
    };
    Some(Color::srgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0))
}

fn parse_hex_color(hex: &str) -> Option<Color> {
    let hex = hex.trim();

    let (r, g, b, a) = match hex.len() {
        3 => {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
            (r, g, b, 255u8)
        }
        4 => {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
            let a = u8::from_str_radix(&hex[3..4].repeat(2), 16).ok()?;
            (r, g, b, a)
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            (r, g, b, 255u8)
        }
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
            (r, g, b, a)
        }
        _ => return None,
    };

    Some(Color::srgba(
        r as f32 / 255.0,
        g as f32 / 255.0,
        b as f32 / 255.0,
        a as f32 / 255.0,
    ))
}

fn parse_color_channel(s: &str) -> Option<f32> {
    let s = s.trim();
    if let Some(pct) = s.strip_suffix('%') {
        return pct.trim().parse::<f32>().ok().map(|n| (n / 100.0).clamp(0.0, 1.0));
    }
    let n: f32 = s.parse().ok()?;
    if n > 1.0 {
        Some((n / 255.0).clamp(0.0, 1.0))
    } else {
        Some(n.clamp(0.0, 1.0))
    }
}

fn parse_alpha_channel(s: &str) -> Option<f32> {
    let s = s.trim();
    if let Some(pct) = s.strip_suffix('%') {
        return pct.trim().parse::<f32>().ok().map(|n| (n / 100.0).clamp(0.0, 1.0));
    }
    s.parse::<f32>().ok().map(|n| n.clamp(0.0, 1.0))
}

fn parse_rgb_color(value: &str) -> Option<Color> {
    let inner = value
        .trim_start_matches("rgba(")
        .trim_start_matches("rgb(")
        .trim_end_matches(')')
        .trim();

    // Modern: rgb(0 0 0 / 0.5) or legacy: rgb(0, 0, 0, 0.5)
    let (channels, alpha) = if let Some((rgb_part, a_part)) = inner.split_once('/') {
        (rgb_part.trim(), Some(a_part.trim()))
    } else {
        (inner, None)
    };

    let parts: Vec<&str> = if channels.contains(',') {
        channels.split(',').map(|s| s.trim()).collect()
    } else {
        channels.split_whitespace().collect()
    };

    let r = parse_color_channel(parts.first()?)?;
    let g = parse_color_channel(parts.get(1)?)?;
    let b = parse_color_channel(parts.get(2)?)?;
    let a = if let Some(a) = alpha {
        parse_alpha_channel(a)?
    } else if parts.len() >= 4 {
        parse_alpha_channel(parts[3])?
    } else {
        1.0
    };

    Some(Color::srgba(r, g, b, a))
}

fn parse_hsl_color(value: &str) -> Option<Color> {
    let inner = value
        .trim_start_matches("hsla(")
        .trim_start_matches("hsl(")
        .trim_end_matches(')')
        .trim();

    let (channels, alpha) = if let Some((hsl_part, a_part)) = inner.split_once('/') {
        (hsl_part.trim(), Some(a_part.trim()))
    } else {
        (inner, None)
    };

    let parts: Vec<&str> = if channels.contains(',') {
        channels.split(',').map(|s| s.trim()).collect()
    } else {
        channels.split_whitespace().collect()
    };

    let h = parse_hue(parts.first()?)?;
    let s = parse_percentage(parts.get(1)?)?;
    let l = parse_percentage(parts.get(2)?)?;
    let a = if let Some(a) = alpha {
        parse_alpha_channel(a)?
    } else if parts.len() >= 4 {
        parse_alpha_channel(parts[3])?
    } else {
        1.0
    };

    let (r, g, b) = hsl_to_rgb(h, s, l);
    Some(Color::srgba(r, g, b, a))
}

fn parse_hue(s: &str) -> Option<f32> {
    let s = s.trim().to_lowercase();
    if let Some(deg) = s.strip_suffix("deg") {
        return deg.trim().parse().ok();
    }
    if let Some(turn) = s.strip_suffix("turn") {
        return turn.trim().parse::<f32>().ok().map(|t| t * 360.0);
    }
    if let Some(rad) = s.strip_suffix("rad") {
        return rad.trim().parse::<f32>().ok().map(|r| r.to_degrees());
    }
    s.parse().ok()
}

fn parse_percentage(s: &str) -> Option<f32> {
    let s = s.trim();
    if let Some(pct) = s.strip_suffix('%') {
        return pct.trim().parse::<f32>().ok().map(|n| (n / 100.0).clamp(0.0, 1.0));
    }
    s.parse::<f32>().ok().map(|n| {
        if n > 1.0 {
            (n / 100.0).clamp(0.0, 1.0)
        } else {
            n.clamp(0.0, 1.0)
        }
    })
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    let h = ((h % 360.0) + 360.0) % 360.0;
    if s == 0.0 {
        return (l, l, l);
    }
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let hk = h / 360.0;
    let tr = (hk + 1.0 / 3.0).rem_euclid(1.0);
    let tg = hk.rem_euclid(1.0);
    let tb = (hk - 1.0 / 3.0).rem_euclid(1.0);
    (
        hue_to_rgb(p, q, tr),
        hue_to_rgb(p, q, tg),
        hue_to_rgb(p, q, tb),
    )
}

fn hue_to_rgb(p: f32, q: f32, t: f32) -> f32 {
    let t = if t < 0.0 {
        t + 1.0
    } else if t > 1.0 {
        t - 1.0
    } else {
        t
    };
    if t < 1.0 / 6.0 {
        p + (q - p) * 6.0 * t
    } else if t < 1.0 / 2.0 {
        q
    } else if t < 2.0 / 3.0 {
        p + (q - p) * (2.0 / 3.0 - t) * 6.0
    } else {
        p
    }
}

/// Convert StyleProps to Bevy's Node component
pub fn json_to_style(props: &StyleProps) -> Node {
    let mut style = Node::default();

    // Sizing
    if let Some(ref w) = props.width {
        style.width = parse_val(&w.0);
    }
    if let Some(ref h) = props.height {
        style.height = parse_val(&h.0);
    }
    if let Some(ref w) = props.min_width {
        style.min_width = parse_val(&w.0);
    }
    if let Some(ref h) = props.min_height {
        style.min_height = parse_val(&h.0);
    }
    if let Some(ref w) = props.max_width {
        style.max_width = parse_val(&w.0);
    }
    if let Some(ref h) = props.max_height {
        style.max_height = parse_val(&h.0);
    }
    if let Some(ref ar) = props.aspect_ratio {
        style.aspect_ratio = parse_aspect_ratio(&ar.0);
    }

    // Flexbox
    if let Some(ref fd) = props.flex_direction {
        style.flex_direction = parse_flex_direction(fd);
    }
    if let Some(fg) = props.flex_grow {
        style.flex_grow = fg;
    }
    if let Some(fs) = props.flex_shrink {
        style.flex_shrink = fs;
    }
    if let Some(ref fb) = props.flex_basis {
        style.flex_basis = parse_val(&fb.0);
    }
    if let Some(ref fw) = props.flex_wrap {
        style.flex_wrap = parse_flex_wrap(fw);
    }
    if let Some(ref ai) = props.align_items {
        style.align_items = parse_align_items(ai);
    }
    if let Some(ref a_self) = props.align_self {
        style.align_self = parse_align_self(a_self);
    }
    if let Some(ref ac) = props.align_content {
        style.align_content = parse_align_content(ac);
    }
    if let Some(ref jc) = props.justify_content {
        style.justify_content = parse_justify_content(jc);
    }
    if let Some(ref ji) = props.justify_items {
        style.justify_items = parse_justify_items(ji);
    }
    if let Some(ref js) = props.justify_self {
        style.justify_self = parse_justify_self(js);
    }

    // CSS Grid
    if let Some(ref cols) = props.grid_template_columns {
        style.grid_template_columns = parse_grid_template(cols);
    }
    if let Some(ref rows) = props.grid_template_rows {
        style.grid_template_rows = parse_grid_template(rows);
    }
    if let Some(ref cols) = props.grid_auto_columns {
        style.grid_auto_columns = parse_grid_auto_tracks(cols);
    }
    if let Some(ref rows) = props.grid_auto_rows {
        style.grid_auto_rows = parse_grid_auto_tracks(rows);
    }
    if let Some(ref flow) = props.grid_auto_flow {
        style.grid_auto_flow = parse_grid_auto_flow(flow);
    }
    if let Some(placement) = build_grid_placement(
        props.grid_column.as_ref(),
        props.grid_column_start.as_ref(),
        props.grid_column_end.as_ref(),
    ) {
        style.grid_column = placement;
    }
    if let Some(placement) = build_grid_placement(
        props.grid_row.as_ref(),
        props.grid_row_start.as_ref(),
        props.grid_row_end.as_ref(),
    ) {
        style.grid_row = placement;
    }

    // Margins — multi-value shorthand, then per-side overrides
    if let Some(ref m) = props.margin {
        style.margin = parse_ui_rect_shorthand(&m.0);
    }
    if let Some(ref m) = props.margin_top {
        style.margin.top = parse_val(&m.0);
    }
    if let Some(ref m) = props.margin_right {
        style.margin.right = parse_val(&m.0);
    }
    if let Some(ref m) = props.margin_bottom {
        style.margin.bottom = parse_val(&m.0);
    }
    if let Some(ref m) = props.margin_left {
        style.margin.left = parse_val(&m.0);
    }

    // Padding
    if let Some(ref p) = props.padding {
        style.padding = parse_ui_rect_shorthand(&p.0);
    }
    if let Some(ref p) = props.padding_top {
        style.padding.top = parse_val(&p.0);
    }
    if let Some(ref p) = props.padding_right {
        style.padding.right = parse_val(&p.0);
    }
    if let Some(ref p) = props.padding_bottom {
        style.padding.bottom = parse_val(&p.0);
    }
    if let Some(ref p) = props.padding_left {
        style.padding.left = parse_val(&p.0);
    }

    // Position
    if let Some(ref pos) = props.position {
        style.position_type = parse_position_type(pos);
    }
    if let Some(ref t) = props.top {
        style.top = parse_val(&t.0);
    }
    if let Some(ref r) = props.right {
        style.right = parse_val(&r.0);
    }
    if let Some(ref b) = props.bottom {
        style.bottom = parse_val(&b.0);
    }
    if let Some(ref l) = props.left {
        style.left = parse_val(&l.0);
    }

    // Border — accept `border` or `borderWidth` as uniform width
    if let Some(b) = props.border.as_ref().or(props.border_width.as_ref()) {
        style.border = parse_ui_rect_shorthand(&b.0);
    }
    if let Some(ref b) = props.border_top {
        style.border.top = parse_val(&b.0);
    }
    if let Some(ref b) = props.border_right {
        style.border.right = parse_val(&b.0);
    }
    if let Some(ref b) = props.border_bottom {
        style.border.bottom = parse_val(&b.0);
    }
    if let Some(ref b) = props.border_left {
        style.border.left = parse_val(&b.0);
    }

    // Gap
    if let Some(ref g) = props.gap {
        let parts: Vec<&str> = g.0.split_whitespace().collect();
        match parts.as_slice() {
            [both] => {
                let val = parse_val(both);
                style.row_gap = val;
                style.column_gap = val;
            }
            [row, col] => {
                style.row_gap = parse_val(row);
                style.column_gap = parse_val(col);
            }
            _ => {
                let val = parse_val(&g.0);
                style.row_gap = val;
                style.column_gap = val;
            }
        }
    }
    if let Some(ref g) = props.row_gap {
        style.row_gap = parse_val(&g.0);
    }
    if let Some(ref g) = props.column_gap {
        style.column_gap = parse_val(&g.0);
    }

    // Display
    if let Some(ref d) = props.display {
        style.display = parse_display(d);
    }

    // Overflow — shorthand then per-axis overrides
    if let Some(ref o) = props.overflow {
        style.overflow = parse_overflow(o);
    }
    if let Some(ref ox) = props.overflow_x {
        style.overflow.x = parse_overflow_axis(ox);
    }
    if let Some(ref oy) = props.overflow_y {
        style.overflow.y = parse_overflow_axis(oy);
    }
    if let Some(ref ocm) = props.overflow_clip_margin {
        style.overflow_clip_margin = parse_overflow_clip_margin(ocm);
    }

    style
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_val() {
        assert_eq!(parse_val("100px"), Val::Px(100.0));
        assert_eq!(parse_val("50%"), Val::Percent(50.0));
        assert_eq!(parse_val("auto"), Val::Auto);
        assert_eq!(parse_val("10vw"), Val::Vw(10.0));
        assert_eq!(parse_val("20vh"), Val::Vh(20.0));
        assert_eq!(parse_val("42"), Val::Px(42.0));
        assert_eq!(parse_val("  8px  "), Val::Px(8.0));
        assert_eq!(parse_val("not-a-length"), Val::Auto);
    }

    #[test]
    fn test_parse_color() {
        assert!(parse_color("red").is_some());
        assert!(parse_color("rebeccapurple").is_some());
        assert!(parse_color("cornflowerblue").is_some());
        assert_eq!(parse_color("transparent"), Some(Color::NONE));
        assert!(parse_color("#ff0000").is_some());
        assert!(parse_color("#f00").is_some());
        assert!(parse_color("rgb(255, 0, 0)").is_some());
        assert!(parse_color("rgba(255, 0, 0, 0.5)").is_some());
        assert!(parse_color("not-a-color").is_none());
    }

    #[test]
    fn test_parse_color_hex_alpha() {
        let short = parse_color("#f008").unwrap().to_srgba();
        assert!((short.red - 1.0).abs() < 0.01);
        assert!((short.alpha - 0x88 as f32 / 255.0).abs() < 0.01);

        let long = parse_color("#00ff0080").unwrap().to_srgba();
        assert!(long.green > 0.99);
        assert!((long.alpha - 0x80 as f32 / 255.0).abs() < 0.01);
    }

    #[test]
    fn test_parse_color_modern_rgb() {
        let c = parse_color("rgb(0 0 0 / 0.5)").unwrap();
        let s = c.to_srgba();
        assert!((s.red - 0.0).abs() < 0.01);
        assert!((s.alpha - 0.5).abs() < 0.01);

        let c2 = parse_color("rgb(255 128 0)").unwrap();
        let s2 = c2.to_srgba();
        assert!((s2.red - 1.0).abs() < 0.01);
        assert!((s2.green - 128.0 / 255.0).abs() < 0.01);

        let pct = parse_color("rgb(100% 0% 0% / 50%)").unwrap().to_srgba();
        assert!((pct.red - 1.0).abs() < 0.01);
        assert!((pct.alpha - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_parse_color_hsl() {
        let red = parse_color("hsl(0, 100%, 50%)").unwrap();
        let s = red.to_srgba();
        assert!((s.red - 1.0).abs() < 0.02);
        assert!(s.green < 0.02);
        assert!(s.blue < 0.02);

        let modern = parse_color("hsla(120 100% 50% / 0.5)").unwrap();
        let ms = modern.to_srgba();
        assert!(ms.green > 0.9);
        assert!((ms.alpha - 0.5).abs() < 0.01);

        let deg = parse_color("hsl(240deg 100% 50%)").unwrap().to_srgba();
        assert!(deg.blue > 0.9);
        assert!(deg.red < 0.02);
    }

    #[test]
    fn test_border_width_alias() {
        let props: StyleProps = serde_json::from_str(r#"{"borderWidth": 2}"#).unwrap();
        let style = json_to_style(&props);
        assert_eq!(style.border.left, Val::Px(2.0));
        assert_eq!(style.border.top, Val::Px(2.0));

        let props_border: StyleProps = serde_json::from_str(r#"{"border": 4}"#).unwrap();
        let style_border = json_to_style(&props_border);
        assert_eq!(style_border.border.left, Val::Px(4.0));

        let per_side: StyleProps = serde_json::from_str(
            r#"{"borderWidth": "2px", "borderTop": "8px", "borderLeft": "1px"}"#,
        )
        .unwrap();
        let side = json_to_style(&per_side);
        assert_eq!(side.border.top, Val::Px(8.0));
        assert_eq!(side.border.left, Val::Px(1.0));
        assert_eq!(side.border.right, Val::Px(2.0));
    }

    #[test]
    fn test_margin_padding_shorthand() {
        let props: StyleProps =
            serde_json::from_str(r#"{"margin": "8px 16px", "padding": "1px 2px 3px 4px"}"#)
                .unwrap();
        let style = json_to_style(&props);
        assert_eq!(style.margin.top, Val::Px(8.0));
        assert_eq!(style.margin.bottom, Val::Px(8.0));
        assert_eq!(style.margin.left, Val::Px(16.0));
        assert_eq!(style.margin.right, Val::Px(16.0));
        assert_eq!(style.padding.top, Val::Px(1.0));
        assert_eq!(style.padding.right, Val::Px(2.0));
        assert_eq!(style.padding.bottom, Val::Px(3.0));
        assert_eq!(style.padding.left, Val::Px(4.0));
    }

    #[test]
    fn test_margin_padding_one_and_three_value() {
        let one: StyleProps = serde_json::from_str(r#"{"margin": "12px"}"#).unwrap();
        let style = json_to_style(&one);
        assert_eq!(style.margin, UiRect::all(Val::Px(12.0)));

        let three: StyleProps =
            serde_json::from_str(r#"{"padding": "1px 2px 3px"}"#).unwrap();
        let style = json_to_style(&three);
        assert_eq!(style.padding.top, Val::Px(1.0));
        assert_eq!(style.padding.right, Val::Px(2.0));
        assert_eq!(style.padding.bottom, Val::Px(3.0));
        assert_eq!(style.padding.left, Val::Px(2.0));

        let override_sides: StyleProps = serde_json::from_str(
            r#"{"margin": "4px", "marginTop": "10px", "marginLeft": "20px"}"#,
        )
        .unwrap();
        let style = json_to_style(&override_sides);
        assert_eq!(style.margin.top, Val::Px(10.0));
        assert_eq!(style.margin.left, Val::Px(20.0));
        assert_eq!(style.margin.right, Val::Px(4.0));
    }

    #[test]
    fn test_aspect_ratio_and_overflow_axes() {
        let props: StyleProps = serde_json::from_str(
            r#"{
                "aspectRatio": "16/9",
                "overflowX": "scroll",
                "overflowY": "hidden",
                "overflowClipMargin": "content-box 4px"
            }"#,
        )
        .unwrap();
        let style = json_to_style(&props);
        assert!((style.aspect_ratio.unwrap() - 16.0 / 9.0).abs() < 0.001);
        assert_eq!(style.overflow.x, OverflowAxis::Scroll);
        assert_eq!(style.overflow.y, OverflowAxis::Clip);
        assert_eq!(
            style.overflow_clip_margin.visual_box,
            bevy::ui::OverflowClipBox::ContentBox
        );
        assert!((style.overflow_clip_margin.margin - 4.0).abs() < 0.001);

        let numeric: StyleProps = serde_json::from_str(r#"{"aspectRatio": 1.5}"#).unwrap();
        assert!((json_to_style(&numeric).aspect_ratio.unwrap() - 1.5).abs() < 0.001);

        let shorthand: StyleProps =
            serde_json::from_str(r#"{"overflow": "hidden"}"#).unwrap();
        let style = json_to_style(&shorthand);
        assert_eq!(style.overflow.x, OverflowAxis::Clip);
        assert_eq!(style.overflow.y, OverflowAxis::Clip);
    }

    #[test]
    fn test_grid_template_and_placement() {
        let props: StyleProps = serde_json::from_str(
            r#"{
                "display": "grid",
                "gridTemplateColumns": "1fr 100px auto",
                "gridTemplateRows": "repeat(2, 50px)",
                "gridColumn": "1 / span 2",
                "gridRow": "2 / 4",
                "gridAutoFlow": "row dense"
            }"#,
        )
        .unwrap();
        let style = json_to_style(&props);
        assert_eq!(style.display, Display::Grid);
        assert_eq!(style.grid_template_columns.len(), 3);
        assert_eq!(style.grid_template_rows.len(), 2);
        assert_eq!(style.grid_auto_flow, GridAutoFlow::RowDense);
        assert_eq!(style.grid_column, GridPlacement::start_span(1, 2));
        assert_eq!(style.grid_row, GridPlacement::start_end(2, 4));
    }

    #[test]
    fn test_grid_tracks_fit_content_and_auto() {
        let cols = parse_grid_template("fit-content(120px) min-content max-content 25%");
        assert_eq!(cols.len(), 4);

        let auto_tracks: StyleProps = serde_json::from_str(
            r#"{
                "gridAutoColumns": "1fr 40px auto",
                "gridAutoRows": "min-content",
                "gridAutoFlow": "column dense"
            }"#,
        )
        .unwrap();
        let style = json_to_style(&auto_tracks);
        assert_eq!(style.grid_auto_columns.len(), 3);
        assert_eq!(style.grid_auto_rows.len(), 1);
        assert_eq!(style.grid_auto_flow, GridAutoFlow::ColumnDense);

        assert_eq!(parse_grid_placement("span 3"), GridPlacement::span(3));
        assert_eq!(parse_grid_placement("auto"), GridPlacement::auto());
        assert_eq!(parse_grid_placement("5"), GridPlacement::start(5));
    }

    #[test]
    fn test_grid_line_start_end_props() {
        let props: StyleProps = serde_json::from_str(
            r#"{
                "gridColumnStart": 2,
                "gridColumnEnd": 5,
                "gridRowStart": 1,
                "gridRowEnd": 3
            }"#,
        )
        .unwrap();
        let style = json_to_style(&props);
        assert_eq!(style.grid_column, GridPlacement::start_end(2, 5));
        assert_eq!(style.grid_row, GridPlacement::start_end(1, 3));
    }

    #[test]
    fn test_border_radius_shorthand_and_corners() {
        let props: StyleProps = serde_json::from_str(
            r#"{
                "borderRadius": "8px 16px",
                "borderTopLeftRadius": "4px"
            }"#,
        )
        .unwrap();
        let radius = style_to_border_radius(&props).unwrap();
        assert_eq!(radius.top_left, Val::Px(4.0));
        assert_eq!(radius.top_right, Val::Px(16.0));
        assert_eq!(radius.bottom_right, Val::Px(8.0));
        assert_eq!(radius.bottom_left, Val::Px(16.0));

        let four = parse_border_radius_shorthand("1px 2px 3px 4px");
        assert_eq!(four.top_left, Val::Px(1.0));
        assert_eq!(four.top_right, Val::Px(2.0));
        assert_eq!(four.bottom_right, Val::Px(3.0));
        assert_eq!(four.bottom_left, Val::Px(4.0));

        let one = parse_border_radius_shorthand("9px");
        assert_eq!(one, BorderRadius::all(Val::Px(9.0)));
    }

    #[test]
    fn test_per_side_border_colors() {
        let props: StyleProps = serde_json::from_str(
            r#"{
                "borderColor": "red",
                "borderTopColor": "blue"
            }"#,
        )
        .unwrap();
        let color = style_to_border_color(&props).unwrap();
        assert_eq!(color.top, parse_color("blue").unwrap());
        assert_eq!(color.left, parse_color("red").unwrap());

        let all_sides: StyleProps = serde_json::from_str(
            r#"{
                "borderTopColor": "red",
                "borderRightColor": "green",
                "borderBottomColor": "blue",
                "borderLeftColor": "yellow"
            }"#,
        )
        .unwrap();
        let color = style_to_border_color(&all_sides).unwrap();
        assert_eq!(color.top, parse_color("red").unwrap());
        assert_eq!(color.right, parse_color("green").unwrap());
        assert_eq!(color.bottom, parse_color("blue").unwrap());
        assert_eq!(color.left, parse_color("yellow").unwrap());
    }

    #[test]
    fn test_box_shadow_and_gradient() {
        let props: StyleProps = serde_json::from_str(
            r#"{
                "boxShadow": "2px 4px 8px 0px rgba(0, 0, 0, 0.5)",
                "backgroundGradient": "linear-gradient(to right, red, blue)"
            }"#,
        )
        .unwrap();
        let shadow = style_to_box_shadow(&props).unwrap();
        assert_eq!(shadow.0.len(), 1);
        assert_eq!(shadow.0[0].x_offset, Val::Px(2.0));
        assert_eq!(shadow.0[0].y_offset, Val::Px(4.0));
        assert_eq!(shadow.0[0].blur_radius, Val::Px(8.0));
        assert_eq!(shadow.0[0].spread_radius, Val::Px(0.0));

        let grad = style_to_background_gradient(&props).unwrap();
        assert_eq!(grad.0.len(), 1);

        let via_image: StyleProps = serde_json::from_str(
            r#"{"backgroundImage": "linear-gradient(90deg, red, blue)"}"#,
        )
        .unwrap();
        assert!(style_to_background_gradient(&via_image).is_some());
    }

    #[test]
    fn test_multi_layer_shadow_and_gradient_stops() {
        let multi = parse_box_shadow(
            "1px 2px 3px red, 4px 5px 6px 7px rgba(0, 0, 0, 0.25)",
        )
        .unwrap();
        assert_eq!(multi.0.len(), 2);
        assert_eq!(multi.0[0].x_offset, Val::Px(1.0));
        assert_eq!(multi.0[1].spread_radius, Val::Px(7.0));
        assert!(parse_box_shadow("none").is_none());

        let stops = parse_background_gradient(
            "linear-gradient(45deg, red 0%, blue 50%, green 100%)",
        )
        .unwrap();
        assert_eq!(stops.0.len(), 1);
        if let Gradient::Linear(linear) = &stops.0[0] {
            assert_eq!(linear.stops.len(), 3);
            assert!((linear.angle - 45f32.to_radians()).abs() < 0.001);
        } else {
            panic!("expected linear gradient");
        }

        let corner = parse_background_gradient("linear-gradient(to top left, white, black)");
        assert!(corner.is_some());
        assert!(parse_background_gradient("radial-gradient(circle, red, blue)").is_none());
    }

    #[test]
    fn test_flex_gap_and_position() {
        let props: StyleProps = serde_json::from_str(
            r#"{
                "display": "flex",
                "flexDirection": "row-reverse",
                "flexWrap": "wrap",
                "flexGrow": 1,
                "flexShrink": 0,
                "flexBasis": "50%",
                "alignItems": "center",
                "justifyContent": "space-between",
                "gap": "8px 16px",
                "position": "absolute",
                "top": "10px",
                "left": "20px",
                "width": 100,
                "height": "50%"
            }"#,
        )
        .unwrap();
        let style = json_to_style(&props);
        assert_eq!(style.display, Display::Flex);
        assert_eq!(style.flex_direction, FlexDirection::RowReverse);
        assert_eq!(style.flex_wrap, FlexWrap::Wrap);
        assert_eq!(style.flex_grow, 1.0);
        assert_eq!(style.flex_shrink, 0.0);
        assert_eq!(style.flex_basis, Val::Percent(50.0));
        assert_eq!(style.align_items, AlignItems::Center);
        assert_eq!(style.justify_content, JustifyContent::SpaceBetween);
        assert_eq!(style.row_gap, Val::Px(8.0));
        assert_eq!(style.column_gap, Val::Px(16.0));
        assert_eq!(style.position_type, PositionType::Absolute);
        assert_eq!(style.top, Val::Px(10.0));
        assert_eq!(style.left, Val::Px(20.0));
        assert_eq!(style.width, Val::Px(100.0));
        assert_eq!(style.height, Val::Percent(50.0));
    }

    #[test]
    fn test_parse_props_and_css_value_numbers() {
        let node = parse_props(r#"{"content":"hi","style":{"width":24,"opacity":0.25}}"#);
        assert_eq!(node.content.as_deref(), Some("hi"));
        let style = node.style.unwrap();
        assert_eq!(style.width.as_ref().unwrap().0, "24px");
        assert_eq!(style.opacity.as_ref().unwrap().0, "0.25");

        let bad = parse_props("not-json");
        assert!(bad.style.is_none());
        assert!(bad.content.is_none());
    }

    #[test]
    fn test_text_and_image_helpers() {
        let props: StyleProps = serde_json::from_str(
            r##"{
                "tint": "#ff0000",
                "tintColor": "blue",
                "opacity": 0.5,
                "textAlign": "center",
                "lineHeight": 1.5,
                "lineBreak": "nowrap",
                "textShadow": "2px 3px 0 black",
                "fontFamily": "fonts/FiraSans.ttf",
                "objectFit": "fill",
                "imageSlice": "16"
            }"##,
        )
        .unwrap();

        assert_eq!(style_text_align(&props), Some(Justify::Center));
        assert_eq!(style_line_break(&props), Some(LineBreak::NoWrap));
        let layout = style_text_layout(&props).unwrap();
        assert_eq!(layout.justify, Justify::Center);
        assert_eq!(layout.linebreak, LineBreak::NoWrap);
        let shadow = style_text_shadow(&props).unwrap();
        assert_eq!(shadow.offset, Vec2::new(2.0, 3.0));
        assert_eq!(
            style_line_height(&props),
            Some(LineHeight::RelativeToFont(1.5))
        );
        assert_eq!(parse_line_height("24px"), Some(LineHeight::Px(24.0)));
        assert_eq!(
            parse_line_height("150%"),
            Some(LineHeight::RelativeToFont(1.5))
        );
        assert_eq!(parse_text_align("justify"), Some(Justify::Justified));
        assert_eq!(parse_text_align("start"), Some(Justify::Left));
        assert_eq!(parse_line_break("break-all"), Some(LineBreak::AnyCharacter));
        assert_eq!(
            style_font_family(&props).as_deref(),
            Some("fonts/FiraSans.ttf")
        );
        assert!(parse_font_family("sans-serif").is_none());
        assert_eq!(style_object_fit(&props), Some(NodeImageMode::Stretch));
        assert_eq!(parse_object_fit("contain"), NodeImageMode::Auto);
        assert!(matches!(
            style_image_mode(&props),
            Some(NodeImageMode::Sliced(_))
        ));
        assert!(matches!(
            parse_image_slice("8 16"),
            Some(NodeImageMode::Sliced(_))
        ));
        assert!(style_tint(&props).is_some());
        let tint_only: StyleProps =
            serde_json::from_str(r#"{"tintColor": "lime"}"#).unwrap();
        assert!(style_tint(&tint_only).is_some());
        assert!((style_opacity(&props).unwrap() - 0.5).abs() < 0.001);
        assert_eq!(parse_opacity("75%"), Some(0.75));
    }
}
