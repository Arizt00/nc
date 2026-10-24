#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

cargo test -p ncc-cli release_phase10_supply_chain_security_gate_tests -- --nocapture
"${NCC_RELEASE_TOOLS:-ncc-release-v43}" release-supply-chain --root "$ROOT_DIR"

echo "OK: qa_phase10_p7_v2_0_0 complete"
