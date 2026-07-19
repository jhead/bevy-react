#!/usr/bin/env bash
# Regenerate bridge TypeScript from Rust (ts-rs + command metadata).
#
# Currently covers the HUD example (`examples/hud/ui/src/generated/`).
# Apps can add their own `assert_bridge_typescript_fresh` test the same way —
# see docs/BRIDGE.md.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

FILTER='bridge_types::tests::generated_bridge_typescript_is_fresh'

echo "Generating HUD bridge TypeScript…"
UPDATE_BRIDGE_TYPES=1 cargo test \
  --manifest-path examples/hud/Cargo.toml \
  "$FILTER" \
  -- --exact --nocapture

echo "Verifying freshness (no rewrite)…"
cargo test \
  --manifest-path examples/hud/Cargo.toml \
  "$FILTER" \
  -- --exact --nocapture

echo "OK — examples/hud/ui/src/generated/ is up to date."
