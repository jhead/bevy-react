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

Stacks are enriched when possible:

- Prefer Boa’s `Display` / shadow-stack frames (`at name (url:line:col)`) when
  the VM attached a backtrace
- Fall back to JS `Error.stack` when present
- Append current VM call frames via `CallFrame::position()` (path + line/col
  for Vite-loaded modules)
- React also includes a `Component stack:` section from the error boundary

Bare function-name frames (no location) still cannot be symbolicated.

### Source maps

Examples enable Vite `sourcemap: true`. When a frame includes
`url:line:column`, the host:

1. Canonicalizes the URL (fixes Path-collapsed `http:/`, strips `?t=` HMR query)
2. Fetches the script and honours `//# sourceMappingURL=` (relative, absolute,
   or `data:` inline maps)
3. Falls back to `{url}.map`
4. Caches decoded maps per URL for the process lifetime

Requires the crate `fetch` feature for `http(s)` Vite URLs (`file://` always
works). See `plugin/src/js/sourcemap_enrich.rs`.

**Remaining gap:** if Boa only reports a function name with no path, there is
nothing to map. Prefer throwing real `Error`s so the VM attaches locations.
