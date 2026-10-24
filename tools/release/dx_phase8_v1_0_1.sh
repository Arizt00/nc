#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "[F8] dx-profile smoke"
cargo test -p ncc-cli release_phase8_dx_observability_gate_tests::phase8_dx_profile_reports_pipeline_trace_for_check_and_run -- --nocapture

echo "[F8] dx smoke OK"
