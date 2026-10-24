#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "[F7] resolve smoke"
cargo test -p ncc-cli release_phase7_package_ecosystem_gate_tests::phase7_resolve_lock_and_verify_lock_are_deterministic -- --nocapture

echo "[F7] resolve suite OK"
