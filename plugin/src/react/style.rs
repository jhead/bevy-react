use bevy::prelude::*;
use bevy::ui::{AlignItems, FlexDirection, JustifyContent, PositionType, Val};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

/// A value that can be either a string or a number (for CSS-like properties)
#[derive(Debug, Clone)]
pub struct CssValue(pub String);

impl Default for CssValue {
    fn default() -> Self {
        CssValue(String::new())
    }
}

impl<'de> Deserialize<'de> for CssValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        match value {
            Value::String(s) => Ok(CssValue(s)),
            Value::Number(n) => {
                // Treat numbers as pixel values
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

/// Props structure from React reconciler
#[derive(Debug, Default, Deserialize)]
pub struct NodeProps {
    #[serde(default)]
    pub style: Option<StyleProps>,
    #[serde(default)]
    pub image: Option<String>,
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

    // Border
    pub border: Option<CssValue>,
    pub border_top: Option<CssValue>,
    pub border_right: Option<CssValue>,
    pub border_bottom: Option<CssValue>,
    pub border_left: Option<CssValue>,
    pub border_radius: Option<CssValue>,

    // Gap
    pub gap: Option<CssValue>,
    pub row_gap: Option<CssValue>,
    pub column_gap: Option<CssValue>,

    // Display
    pub display: Option<String>,
    pub overflow: Option<String>,

    // Colors (for BackgroundColor, BorderColor)
    pub background_color: Option<String>,
    pub border_color: Option<String>,

    // Text styling
    pub color: Option<String>,
    pub font_size: Option<CssValue>,
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

    if let Some(px) = value.strip_suffix("px") {
        if let Ok(n) = px.trim().parse::<f32>() {
            return Val::Px(n);
        }
    }

    if let Some(pct) = value.strip_suffix("%") {
        if let Ok(n) = pct.trim().parse::<f32>() {
            return Val::Percent(n);
        }
    }

    if let Some(vw) = value.strip_suffix("vw") {
        if let Ok(n) = vw.trim().parse::<f32>() {
            return Val::Vw(n);
        }
    }

    if let Some(vh) = value.strip_suffix("vh") {
        if let Ok(n) = vh.trim().parse::<f32>() {
            return Val::Vh(n);
        }
    }

    // Try parsing as plain number (treat as px)
    if let Ok(n) = value.parse::<f32>() {
        return Val::Px(n);
    }

    log::warn!("Unknown Val format: '{}', defaulting to Auto", value);
    Val::Auto
}

/// Parse flex direction
fn parse_flex_direction(value: &str) -> FlexDirection {
    match value.to_lowercase().as_str() {
        "row" => FlexDirection::Row,
        "row-reverse" | "rowreverse" => FlexDirection::RowReverse,
        "column" | "col" => FlexDirection::Column,
        "column-reverse" | "columnreverse" => FlexDirection::ColumnReverse,
        _ => FlexDirection::default(),
    }
}

/// Parse align items
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

/// Parse justify content
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

/// Parse position type
fn parse_position_type(value: &str) -> PositionType {
    match value.to_lowercase().as_str() {
        "relative" => PositionType::Relative,
        "absolute" => PositionType::Absolute,
        _ => PositionType::default(),
    }
}

/// Parse a CSS color string to Bevy Color
/// Supports: "red", "blue", "#ff0000", "rgb(255, 0, 0)", "rgba(255, 0, 0, 1.0)"
pub fn parse_color(value: &str) -> Option<Color> {
    let value = value.trim().to_lowercase();

    // Named colors
    match value.as_str() {
        "transparent" => return Some(Color::NONE),
        "black" => return Some(Color::BLACK),
        "white" => return Some(Color::WHITE),
        "red" => return Some(Color::srgb(1.0, 0.0, 0.0)),
        "green" => return Some(Color::srgb(0.0, 1.0, 0.0)),
        "blue" => return Some(Color::srgb(0.0, 0.0, 1.0)),
        "yellow" => return Some(Color::srgb(1.0, 1.0, 0.0)),
        "cyan" => return Some(Color::srgb(0.0, 1.0, 1.0)),
        "magenta" => return Some(Color::srgb(1.0, 0.0, 1.0)),
        "gray" | "grey" => return Some(Color::srgb(0.5, 0.5, 0.5)),
        "darkgray" | "darkgrey" => return Some(Color::srgb(0.25, 0.25, 0.25)),
        "lightgray" | "lightgrey" => return Some(Color::srgb(0.75, 0.75, 0.75)),
        "orange" => return Some(Color::srgb(1.0, 0.65, 0.0)),
        "pink" => return Some(Color::srgb(1.0, 0.75, 0.8)),
        "purple" => return Some(Color::srgb(0.5, 0.0, 0.5)),
        "brown" => return Some(Color::srgb(0.6, 0.3, 0.0)),
        _ => {}
    }

    // Hex color: #RGB, #RGBA, #RRGGBB, #RRGGBBAA
    if let Some(hex) = value.strip_prefix("#") {
        return parse_hex_color(hex);
    }

    // rgb(r, g, b) or rgba(r, g, b, a)
    if value.starts_with("rgb") {
        return parse_rgb_color(&value);
    }

    log::warn!("Unknown color format: '{}'", value);
    None
}

fn parse_hex_color(hex: &str) -> Option<Color> {
    let hex = hex.trim();

    let (r, g, b, a) = match hex.len() {
        3 => {
            // #RGB -> #RRGGBB
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
            (r, g, b, 255u8)
        }
        4 => {
            // #RGBA -> #RRGGBBAA
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
            let a = u8::from_str_radix(&hex[3..4].repeat(2), 16).ok()?;
            (r, g, b, a)
        }
        6 => {
            // #RRGGBB
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            (r, g, b, 255u8)
        }
        8 => {
            // #RRGGBBAA
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

fn parse_rgb_color(value: &str) -> Option<Color> {
    // Remove "rgb(" or "rgba(" and ")"
    let inner = value
        .trim_start_matches("rgba(")
        .trim_start_matches("rgb(")
        .trim_end_matches(")");

    let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();

    let r: f32 = parts.first()?.parse().ok()?;
    let g: f32 = parts.get(1)?.parse().ok()?;
    let b: f32 = parts.get(2)?.parse().ok()?;
    let a: f32 = parts.get(3).and_then(|s| s.parse().ok()).unwrap_or(1.0);

    // Normalize to 0-1 range if values are 0-255
    let (r, g, b) = if r > 1.0 || g > 1.0 || b > 1.0 {
        (r / 255.0, g / 255.0, b / 255.0)
    } else {
        (r, g, b)
    };

    Some(Color::srgba(r, g, b, a))
}

/// Convert StyleProps to Bevy's Node (formerly Style) component
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
    if let Some(ref ai) = props.align_items {
        style.align_items = parse_align_items(ai);
    }
    if let Some(ref jc) = props.justify_content {
        style.justify_content = parse_justify_content(jc);
    }

    // Margins - handle shorthand first
    if let Some(ref m) = props.margin {
        let val = parse_val(&m.0);
        style.margin = UiRect::all(val);
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

    // Padding - handle shorthand first
    if let Some(ref p) = props.padding {
        let val = parse_val(&p.0);
        style.padding = UiRect::all(val);
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

    // Border
    if let Some(ref b) = props.border {
        let val = parse_val(&b.0);
        style.border = UiRect::all(val);
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
    if let Some(ref g) = props.row_gap {
        style.row_gap = parse_val(&g.0);
    }
    if let Some(ref g) = props.column_gap {
        style.column_gap = parse_val(&g.0);
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
    }

    #[test]
    fn test_parse_color() {
        assert!(parse_color("red").is_some());
        assert!(parse_color("#ff0000").is_some());
        assert!(parse_color("#f00").is_some());
        assert!(parse_color("rgb(255, 0, 0)").is_some());
        assert!(parse_color("rgba(255, 0, 0, 0.5)").is_some());
    }
}

