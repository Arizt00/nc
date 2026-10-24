#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "[F10-P14] ci/cd compatibility gate"
cargo test -p ncc-cli release_phase10_cicd_compat_gate_tests -- --nocapture
"${NCC_RELEASE_TOOLS:-ncc-release-v43}" release-cicd-compat --root "$ROOT_DIR"

echo "[F10-P14] OK"
