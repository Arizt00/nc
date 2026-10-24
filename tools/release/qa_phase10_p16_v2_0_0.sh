#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "[F10-P16] installer matrix gate"
cargo test -p ncc-cli release_phase10_installer_matrix_gate_tests -- --nocapture
"${NCC_RELEASE_TOOLS:-ncc-release-v43}" release-installer-matrix --root "$ROOT_DIR"

echo "[F10-P16] OK"
