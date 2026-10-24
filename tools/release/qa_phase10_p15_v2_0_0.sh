#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "[F10-P15] telemetry gate"
cargo test -p ncc-cli release_phase10_telemetry_gate_tests -- --nocapture
"${NCC_NATIVE_TOOLS:-ncc-tools-v43}" dist-telemetry --root "$ROOT_DIR" \
  --report "$ROOT_DIR/docs/status/F10_P15_telemetry_report.json"

echo "[F10-P15] OK"
