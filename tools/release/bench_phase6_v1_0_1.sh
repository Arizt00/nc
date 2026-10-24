#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "[F6] benchmark smoke + reproducibility"
cargo test -p ncc-cli release_phase6_quality_extreme_gate_tests::phase6_benchmark_smoke_and_reproducible_outputs_stay_stable -- --nocapture

echo "[F6] bench suite OK"
