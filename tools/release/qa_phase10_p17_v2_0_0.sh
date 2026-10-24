#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "[F10-P17] support policy gate"
cargo test -p ncc-cli release_phase10_support_policy_gate_tests -- --nocapture
"${NCC_NATIVE_TOOLS:-ncc-tools-v43}" release-policy --root "$ROOT_DIR"

echo "[F10-P17] OK"
