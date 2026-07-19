#!/usr/bin/env bash
# Automated demo smoke: build TS, run plugin tests, production-build demo UI,
# cargo-check demo, optionally launch release demo briefly (no Vite required).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

LAUNCH_SECONDS="${DEMO_SMOKE_LAUNCH_SECONDS:-5}"
SKIP_LAUNCH="${DEMO_SMOKE_SKIP_LAUNCH:-0}"

echo "==> Build packages/bevy-react"
(cd packages/bevy-react && pnpm build)

echo "==> Plugin tests"
cargo test --manifest-path plugin/Cargo.toml --quiet

echo "==> Production-build examples/demo/ui"
(cd examples/demo/ui && pnpm install --ignore-scripts --frozen-lockfile 2>/dev/null || pnpm install --ignore-scripts)
(cd examples/demo/ui && pnpm build)

echo "==> cargo check examples/demo"
cargo check --manifest-path examples/demo/Cargo.toml --quiet

if [[ "$SKIP_LAUNCH" == "1" ]]; then
  echo "==> Skipping brief release launch (DEMO_SMOKE_SKIP_LAUNCH=1)"
else
  echo "==> Brief release demo launch (${LAUNCH_SECONDS}s, then kill)"
  # Release path loads ui/dist/app.js — Vite not required.
  cargo run --manifest-path examples/demo/Cargo.toml --release &
  demo_pid=$!
  cleanup() {
    if kill -0 "$demo_pid" 2>/dev/null; then
      kill "$demo_pid" 2>/dev/null || true
      wait "$demo_pid" 2>/dev/null || true
    fi
  }
  trap cleanup EXIT
  sleep "$LAUNCH_SECONDS"
  cleanup
  trap - EXIT
  echo "==> Demo process exited after smoke window"
fi

echo "OK — automated smoke passed. For click/hover/focus, see docs/DEMO_SMOKE.md"
