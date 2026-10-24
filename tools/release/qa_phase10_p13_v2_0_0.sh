#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "[F10-P13] first-run gate"
cargo test -p ncc-cli release_phase10_first_run_gate_tests -- --nocapture
"${NCC_NATIVE_TOOLS:-ncc-tools-v43}" release-first-run --root "$ROOT_DIR"

echo "[F10-P13] OK"
