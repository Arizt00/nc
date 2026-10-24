#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "[F9] release-train-model smoke"
cargo test -p ncc-cli release_train_model_cli_tests -- --nocapture

echo "[F9] release-train-model command list"
cargo run -p ncc-cli -- release-train-model --list

echo "[F9] promotion pipeline dry-run"
NCC_SKIP_PROMOTION_GATES=1 tools/release/promote_release_train_v1_0_1.sh nightly --candidate local-smoke --dry-run

echo "[F9] release-train smoke OK"
