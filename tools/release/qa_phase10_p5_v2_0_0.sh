#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

cargo test -p ncc-cli release_phase10_ncup_commands_gate_tests -- --nocapture
"${NCC_NATIVE_TOOLS:-ncc-tools-v43}" release-ncup-commands --root "$ROOT_DIR"

echo "OK: qa_phase10_p5_v2_0_0 complete"
