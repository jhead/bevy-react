# Boa / JS compatibility notes

bevy-react runs app code inside [Boa](https://github.com/boa-dev/boa), not a browser.
Many Web/Node APIs are missing or partial. Prefer portable JS.

## Known gaps

| API | Status | Notes |
|-----|--------|--------|
| `setTimeout` / `setInterval` / `rAF` | Shimmed | Via `boa_runtime` timers + rAF→setTimeout |
| `MessageChannel` | Shimmed | React scheduler |
| `fetch` | Feature-gated | Enable crate `fetch` feature |
| `console.*` | Forwarded | Rust `log` + `JsRuntimeError` |
| `Date#toLocaleString` / `toLocaleDateString` / `toLocaleTimeString` | **Shimmed fallback** | Boa Intl is unimplemented; shims use ISO / simple time |
| `Intl.*` | Minimal stub | `DateTimeFormat` / `NumberFormat` fallbacks only |
| `navigator.clipboard` | Missing | TextInput uses in-process clipboard |
| DOM (`document`, `window` layout, etc.) | N/A | No DOM — use bevy-react host components |

## Guidance

- Avoid `toLocaleString` options that need full ICU; the shim is best-effort.
- Do not assume `window`/`document` beyond the tiny location/process shims.
- Prefer `ReactBridge` / `callNative` for game state instead of inventing globals.

If you hit a `"Function Unimplemented"` error from Boa, file it here or add a shim in `plugin/src/react/shim.rs`.
