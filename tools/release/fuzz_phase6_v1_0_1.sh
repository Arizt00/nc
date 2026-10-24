#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "[F6] fuzz parser/sema/runtime (deterministic mutation)"
cargo test -p ncc-cli release_phase6_quality_extreme_gate_tests::phase6_parser_sema_runtime_mutation_fuzz_does_not_panic -- --nocapture

echo "[F6] fuzz suite OK"
