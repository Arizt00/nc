#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "[F8] dx + observability gate"
cargo test -p ncc-cli release_phase8_dx_observability_gate_tests -- --nocapture

echo "[F8] regression gates"
cargo test -p ncc-cli release_phase7_package_ecosystem_gate_tests -- --nocapture
cargo test -p ncc-cli release_phase6_quality_extreme_gate_tests -- --nocapture

echo "[F8] dx + observability suite OK"
