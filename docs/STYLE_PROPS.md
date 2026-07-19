# Style Props

CSS-like styles are passed on component `style` props, serialized to JSON, and converted on the Rust side in `plugin/src/react/style.rs` (layout) plus `plugin/src/react/systems/render.rs` (colors, radius, z-index, text).

TypeScript types live in `packages/bevy-react/src/types.ts` (`BevyStyle`). The two sides are **not fully aligned** yet — see [Known drift](#known-drift).

## Value types

| Kind | Accepted forms |
|---|---|
| Length (`width`, `margin`, …) | number (treated as `px`), `"100px"`, `"50%"`, `"auto"`, `"10vw"`, `"20vh"` |
| Color | named (see below), `#RGB` / `#RGBA` / `#RRGGBB` / `#RRGGBBAA`, `rgb(...)`, `rgba(...)` |
| Enums | string keywords as listed per property |

**Named colors (Rust):** `transparent`, `black`, `white`, `red`, `green`, `blue`, `yellow`, `cyan`, `magenta`, `gray`/`grey`, `darkgray`/`darkgrey`, `lightgray`/`lightgrey`, `orange`, `pink`, `purple`, `brown`.

Shorthands for `margin` / `padding` / `border` / `gap` / `borderRadius` currently apply a **single** value to all sides (multi-value CSS shorthands like `"8px 16px"` are not supported yet).

## Layout props → Bevy `Node`

These are applied via `json_to_style` in Rust.

| Prop (camelCase) | Notes / accepted values |
|---|---|
| `width`, `height` | Length |
| `minWidth`, `minHeight` | Length |
| `maxWidth`, `maxHeight` | Length |
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
| `margin` | Length → all sides |
| `marginTop`, `marginRight`, `marginBottom`, `marginLeft` | Length (override shorthand) |
| `padding` | Length → all sides |
| `paddingTop`, `paddingRight`, `paddingBottom`, `paddingLeft` | Length |
| `position` | `relative`, `absolute` |
| `top`, `right`, `bottom`, `left` | Length |
| `border` | Length → border width on all sides |
| `borderTop`, `borderRight`, `borderBottom`, `borderLeft` | Length (override shorthand) |
| `gap` | Length → both row and column gap |
| `rowGap`, `columnGap` | Length |
| `display` | Rust: `flex`, `none`, `grid`, `block`. TS types currently only list `flex` \| `none`. `grid` parses but grid template props are not implemented. |
| `overflow` | `visible`, `clip` / `hidden`, `scroll` (applies to both axes) |

## Visual / text props (applied outside `Node`)

| Prop | Effect |
|---|---|
| `backgroundColor` | `BackgroundColor` |
| `borderColor` | `BorderColor` |
| `borderRadius` | `BorderRadius::all` (single value) |
| `zIndex` | `ZIndex` |
| `color` | `TextColor` (text nodes) |
| `fontSize` | `TextFont` size (text nodes) |

## Known drift

| Topic | Detail |
|---|---|
| Border width | TS declares `borderWidth`; Rust expects `border` / `borderTop` / … — `borderWidth` is currently ignored by Rust. Prefer `border` until Epic 2 reconciles the contracts. |
| Per-side borders | Rust supports `borderTop` etc.; TS `BevyStyle` does not list them. |
| Display | Rust accepts `grid` / `block`; TS union is narrower. |
| Text styles | `color` / `fontSize` are on `StyleProps` in Rust and on `TextProps.style` in TS. |
| `fontFamily` | Declared on TS `Text` styles as unsupported. |
| Clearing props | Removing a style key on update may leave the previous Bevy component value (tracked in the project plan). |

## Planned (not implemented)

From [PROJECT_PLAN.md](PROJECT_PLAN.md) Epic 2: multi-value margin/padding/radius shorthands, fuller color parsing (`hsl`, modern `rgb` syntax), per-corner radius, per-side border colors, opacity, box shadow, gradients, grid templates, `aspectRatio`, axis-specific overflow, richer text layout (`textAlign`, `lineHeight`, fonts via assets), image `objectFit` / tint.
