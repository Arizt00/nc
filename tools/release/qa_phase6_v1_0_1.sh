#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "[F6] quality extreme gate"
cargo test -p ncc-cli release_phase6_quality_extreme_gate_tests -- --nocapture

echo "[F6] regression critical gates"
cargo test -p ncc-cli release_phase1_language_base_gate_tests -- --nocapture
cargo test -p ncc-cli release_phase2_toolchain_model_gate_tests -- --nocapture
cargo test -p ncc-cli release_phase3_runtime_stdlib_gate_tests -- --nocapture
cargo test -p ncc-cli release_phase4_backend_sovereign_gate_tests -- --nocapture
cargo test -p ncc-cli release_phase5_toolchain_complete_gate_tests -- --nocapture

echo "[F6] quality gate suite OK"
