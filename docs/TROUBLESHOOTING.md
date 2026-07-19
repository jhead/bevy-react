# Troubleshooting

## Errors appear in-game

`JsPlugin` shows a built-in **error overlay** whenever the JS engine reports a
failure into `JsRuntimeError`. That includes:

- `console.error`
- Uncaught promise rejections
- Script / module / job failures
- React render failures (`BevyErrorBoundary` / `__react_report_error`)

The overlay is a high `GlobalZIndex` Bevy UI panel with the message and stack.
Dismiss it with the **Dismiss** button or **Escape**.

You can also read `Res<JsRuntimeError>` yourself if you want a custom UI.

### How to trigger it

Throw from a React component (or call `reportErrorToHost`):

```tsx
function Boom() {
  throw new Error("intentional failure");
}
```

Or from an async module load path — failed `import()` / missing default export
already logs via `console.error` and feeds the same overlay.

### What stacks look like

Stacks are best-effort:

- Prefer the JS `Error.stack` when Boa provides one
- Append Boa VM frame names when available
- React also includes a `Component stack:` section from the error boundary

Boa often emits VM-style frames (function names) rather than full
`file:line:column` browser stacks.

### Source maps

Examples enable Vite `sourcemap: true`. The host tries to rewrite frames that
look like `url:line:column` by fetching `{url}.map` (file:// or http(s) with
the crate `fetch` feature).

**Limitation:** many Boa stacks lack URL locations, so maps may not apply.
When a frame *does* include a Vite URL, mapping is attempted; otherwise the
raw stack is shown. See `plugin/src/js/sourcemap_enrich.rs` for the hook /
TODO (cache maps, honour `sourceMappingURL`, fuller Vite HMR symbolication).
