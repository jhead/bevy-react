# Style Props

CSS-like styles are passed on component `style` props, serialized to JSON, and converted on the Rust side in `plugin/src/react/style.rs` (layout + parsers) plus `plugin/src/react/systems/render.rs` (colors, radius, z-index, text, and other visual components).

TypeScript types live in `packages/bevy-react/src/types.ts` (`BevyStyle`).

## Value types

| Kind | Accepted forms |
|---|---|
| Length (`width`, `margin`, …) | number (treated as `px`), `"100px"`, `"50%"`, `"auto"`, `"10vw"`, `"20vh"` |
| Scalar (`aspectRatio`, `lineHeight`, `opacity`) | number (no implied `px`), or string (`"16/9"`, `"1.5"`, `"50%"`) |
| Color | named (CSS Level 1–3 table), `#RGB` / `#RGBA` / `#RRGGBB` / `#RRGGBBAA`, `rgb`/`rgba` (legacy commas or modern space/`/` syntax), `hsl`/`hsla` |
| Enums | string keywords as listed per property |

**Shorthands:** `margin` / `padding` / `border` / `borderRadius` / `gap` accept 1–4 CSS values (e.g. `"8px 16px"`). Per-side / per-corner props override the shorthand.

## Layout props → Bevy `Node`

Applied via `json_to_style` (used by the render system).

| Prop (camelCase) | Notes / accepted values |
|---|---|
| `width`, `height` | Length |
| `minWidth`, `minHeight` | Length |
| `maxWidth`, `maxHeight` | Length |
| `aspectRatio` | number or `"16/9"` → `Node::aspect_ratio` |
| `flexDirection` | `row`, `column`, `row-reverse` / `rowReverse`, `column-reverse` / `columnReverse`, `col` |
| `flexWrap` | `nowrap` / `noWrap` / `no-wrap`, `wrap`, `wrap-reverse` / `wrapReverse` |
| `flexGrow`, `flexShrink` | number |
| `flexBasis` | Length |
| `alignItems` | `start` / `flex-start`, `end` / `flex-end`, `center`, `baseline`, `stretch` |
| `alignSelf` | `auto`, plus same as `alignItems` |
| `alignContent` | `start` / `flex-start`, `end` / `flex-end`, `center`, `stretch`, `space-between` / `spaceBetween`, `space-around` / `spaceAround`, `space-evenly` / `spaceEvenly` |
| `justifyContent` | `start` / `flex-start`, `end` / `flex-end`, `center`, `space-between` / `spaceBetween`, `space-around` / `spaceAround`, `space-evenly` / `spaceEvenly` |
| `justifyItems` | `start` / `flex-start`, `end` / `flex-end`, `center`, `baseline`, `stretch` |
| `justifySelf` | `auto`, plus same as `justifyItems` |
| `gridTemplateColumns`, `gridTemplateRows` | Track list: `1fr`, `100px`, `auto`, `min-content`, `max-content`, `%`, `fit-content(…)`, `repeat(N, track)` |
| `gridAutoColumns`, `gridAutoRows` | Space-separated track sizes |
| `gridAutoFlow` | `row`, `column`, `row dense`, `column dense` |
| `gridColumn`, `gridRow` | Placement: `"1"`, `"1 / 3"`, `"span 2"`, `"1 / span 2"` |
| `gridColumnStart/End`, `gridRowStart/End` | Line indexes (override shorthand when set alone) |
| `margin` | 1–4 value shorthand → `UiRect` |
| `marginTop`, `marginRight`, `marginBottom`, `marginLeft` | Length (override shorthand) |
| `padding` | 1–4 value shorthand → `UiRect` |
| `paddingTop`, `paddingRight`, `paddingBottom`, `paddingLeft` | Length |
| `position` | `relative`, `absolute` |
| `top`, `right`, `bottom`, `left` | Length |
| `border` / `borderWidth` | 1–4 value shorthand → border width |
| `borderTop`, `borderRight`, `borderBottom`, `borderLeft` | Length (override shorthand) |
| `gap` | 1 value (both axes) or 2 values (`row column`) |
| `rowGap`, `columnGap` | Length |
| `display` | `flex`, `none`, `grid`, `block` |
| `overflow` | `visible`, `clip` / `hidden`, `scroll` (both axes) |
| `overflowX`, `overflowY` | Per-axis overflow (override shorthand) |
| `overflowClipMargin` | `content-box` / `padding-box` / `border-box`, optional `px` margin (`"content-box 4px"`), or bare length |

## Visual / text / image helpers

Parsers and builders live in `style.rs`. Layout-independent props are applied by `render.rs` (some helpers below still need render wiring — see [Render wiring](#render-wiring)).

| Prop | Effect / helper |
|---|---|
| `backgroundColor` | `BackgroundColor` via `parse_color` |
| `borderColor` | Uniform border; use `style_to_border_color` for per-side |
| `borderTopColor`, `borderRightColor`, `borderBottomColor`, `borderLeftColor` | Per-side via `style_to_border_color` |
| `borderRadius` | 1–4 value shorthand via `style_to_border_radius` |
| `borderTopLeftRadius`, `borderTopRightRadius`, `borderBottomRightRadius`, `borderBottomLeftRadius` | Per-corner overrides |
| `zIndex` | `ZIndex` |
| `color` | `TextColor` (text nodes) |
| `fontSize` | `TextFont` size (text nodes) |
| `fontFamily` | Asset path string via `parse_font_family` → `AssetServer::load` in `apply_text_style*` (generic CSS families ignored) |
| `textAlign` | `Justify` via `parse_text_align` (`left`/`start`, `right`/`end`, `center`, `justify`) |
| `lineHeight` | `LineHeight` via `parse_line_height` (unitless → `RelativeToFont`, `px` → `Px`) |
| `pointerEvents` | `"none"` → `Pickable::IGNORE` + `FocusPolicy::Pass` (HUD pass-through); `"auto"` → default blocking |
| `opacity` | `0`–`1` or `%` via `style_opacity` (no Bevy `UiOpacity`; multiply into colors) |
| `boxShadow` | `BoxShadow` via `parse_box_shadow` / `style_to_box_shadow` |
| `backgroundImage` / `backgroundGradient` | `linear-gradient(...)` → `BackgroundGradient` via `style_to_background_gradient` |
| `objectFit` | `NodeImageMode` via `parse_object_fit` (`fill`/`stretch` → Stretch; others → Auto) |
| `tint` / `tintColor` | Image tint via `style_tint` |

## Interaction styles (host-side)

Hover / pressed / focused / checked visuals are applied entirely in Rust from Bevy
`Interaction`, picking `Hovered`, focus state, and the UI `Checked` marker — no React
re-render required. JS still owns `onClick` and other event handlers.

```tsx
style={{
  backgroundColor: '#333',
  hover: { backgroundColor: '#555' },
  pressed: { backgroundColor: '#222' },
  focused: { borderColor: '#4af' },
  checked: { backgroundColor: '#5a5aff' }, // e.g. Checkbox
  transition: 'backgroundColor 100ms', // or { backgroundColor: 100 }
}}
```

Merge order (later wins): **base → checked → focused → hover → pressed**.

| Prop | Effect |
|---|---|
| `hover` | Nested `BevyStyle` when `Interaction` is hovered/pressed **or** picking `Hovered(true)` |
| `pressed` | Nested overrides when `Interaction::Pressed` |
| `focused` | Nested overrides when the node has keyboard/input focus |
| `checked` | Nested overrides when Bevy UI `Checked` is present (checkbox / toggles) |
| `transition` | Host-side lerp for `backgroundColor`, `borderColor`, `color`, `opacity` (string or `{ prop: ms }`) |

Unknown style keys are logged with `log::warn` at parse time (`Unsupported style prop '…'`) instead of being silently dropped.

## Render wiring

These are **parsed and typed** in `style.rs` / `BevyStyle`, and layout props already flow through `json_to_style`. Visual helpers still need `render.rs` to call them for full end-to-end effect:

- Per-corner `borderRadius` / per-side `border*Color` (render still uses `BorderRadius::all` / `BorderColor::all` on the uniform props)
- `opacity`, `boxShadow`, `BackgroundGradient`
- Image `objectFit`, `tint`

Wired end-to-end: `fontFamily`, `textAlign`, `lineHeight`, `pointerEvents`.

`parse_color` extensions (named colors, HSL, modern `rgb`) apply immediately wherever render already calls `parse_color`.

## Fonts

| Topic | Detail |
|---|---|
| `fontFamily` | Pass a Bevy asset path such as `"fonts/FiraSans.ttf"` (file under your game's `assets/`). Generic CSS names (`sans-serif`, `monospace`, …) are ignored. |
| Default font / tofu | With no `fontFamily` (and no default handle), Bevy uses its built-in **FiraMono subset**. Missing glyphs render as tofu (□) — e.g. Unicode minus `−`. Prefer ASCII `+/-` or a full font asset. |
| Plugin / root default | `ReactDefaultFont` resource (or `ReactDefaultFontPlugin::new("fonts/…")`), or attach `ReactRootFont(handle)` on the root entity. Resolution order: `fontFamily` → root font → plugin default → Bevy subset. |

## Known limitations

| Topic | Detail |
|---|---|
| `objectFit` | Bevy `NodeImageMode` is Auto / Stretch / Sliced / Tiled — CSS `contain`/`cover` map to Auto |
| `opacity` | No dedicated Bevy UI opacity component in 0.17 |
| Grid `minmax()` / `auto-fill` | Not fully parsed; `repeat(N, track)` integer repeats work |
| Atlas / nine-slice | Not exposed yet |
| Text shadow / line-break | Parsed; confirm render coverage if you rely on them |
| `pointerEvents` | Per-node only (like CSS with explicit `auto` on children). Full-screen HUD: set `"none"` on the transparent overlay; keep `"auto"` / default on interactive widgets. |
