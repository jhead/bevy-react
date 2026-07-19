#!/usr/bin/env bash
# Regenerate bridge TypeScript from Rust (ts-rs + command metadata).
#
# Writes into `packages/bridge-types/src/` (HUD `PlayerStats` + command wrappers).
# Apps can add their own `assert_bridge_typescript_fresh` test the same way —
# see docs/BRIDGE.md.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

FILTER='bridge_types::tests::generated_bridge_typescript_is_fresh'

echo "Generating bridge TypeScript into packages/bridge-types/src/…"
UPDATE_BRIDGE_TYPES=1 cargo test \
  --manifest-path examples/hud/Cargo.toml \
  "$FILTER" \
  -- --exact --nocapture

echo "Verifying freshness (no rewrite)…"
cargo test \
  --manifest-path examples/hud/Cargo.toml \
  "$FILTER" \
  -- --exact --nocapture

echo "OK — packages/bridge-types/src/ is up to date."
