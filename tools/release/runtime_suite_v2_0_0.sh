#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "[v2.0.0/FaseB] workspace check"
cargo check --workspace

echo "[v2.0.0/FaseB] runtime fixture matrix tests for mode execution"
cargo test -q -p ncc-cli --lib runtime_f14_network_fixture_matrix_tests -- --nocapture
cargo test -q -p ncc-cli --lib runtime_f15_system_fixture_matrix_tests -- --nocapture
cargo test -q -p ncc-cli --lib runtime_f16_tooling_fixture_matrix_tests -- --nocapture
cargo test -q -p ncc-cli --lib runtime_f17_compiler_fixture_matrix_tests -- --nocapture
cargo test -q -p ncc-cli --lib runtime_f18_kernel_fixture_matrix_tests -- --nocapture
cargo test -q -p ncc-cli --lib runtime_f19_ai_fixture_matrix_tests -- --nocapture

echo "[v2.0.0/FaseB] execution smoke for user mode suite"
for f in examples/user_v2_0_0/*.n; do
  echo "  - check $f"
  cargo run -q -p ncc-cli --bin ncc -- check "$f"
  echo "  - run   $f"
  cargo run -q -p ncc-cli --bin ncc -- run "$f"
done

echo "[v2.0.0/FaseB] runtime suite OK"
