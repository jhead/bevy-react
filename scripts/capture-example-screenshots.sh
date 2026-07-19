#!/usr/bin/env bash
# Build each example UI (release path) and capture a Bevy window screenshot
# into docs/media/. Requires a display. Uses BEVY_REACT_SCREENSHOT in examples.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

MEDIA="$ROOT/docs/media"
mkdir -p "$MEDIA"

DELAY="${BEVY_REACT_SCREENSHOT_DELAY:-3.5}"

echo "==> Build packages/bevy-react"
(cd packages/bevy-react && pnpm build)

capture_one() {
  local name="$1"
  local out="$MEDIA/${name}.png"
  echo "==> Capture $name → $out"

  (cd "examples/${name}/ui" && pnpm install --ignore-scripts >/dev/null && pnpm build)

  rm -f "$out"
  BEVY_REACT_SCREENSHOT="$out" \
  BEVY_REACT_SCREENSHOT_DELAY="$DELAY" \
    cargo run --manifest-path "examples/${name}/Cargo.toml" --release

  if [[ ! -f "$out" ]]; then
    echo "ERROR: missing screenshot $out" >&2
    exit 1
  fi
  echo "    ok ($(wc -c < "$out") bytes)"
}

# Sequential — one Bevy window at a time on the shared display.
for example in demo menu forms hud; do
  capture_one "$example"
done

echo "OK — screenshots in $MEDIA"
ls -la "$MEDIA"/*.png
