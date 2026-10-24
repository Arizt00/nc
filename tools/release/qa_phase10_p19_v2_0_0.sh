#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "[F10-P19] final closure gate"
cargo test -p ncc-cli release_phase10_final_closure_gate_tests -- --nocapture
"${NCC_NATIVE_TOOLS:-ncc-tools-v43}" release-final --root "$ROOT_DIR"

echo "[F10-P19] OK"
