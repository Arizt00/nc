#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

cargo test -p ncc-cli release_phase10_manifest_format_gate_tests -- --nocapture
"${NCC_RELEASE_TOOLS:-ncc-release-v43}" release-manifest-format --root "$ROOT_DIR"

echo "OK: qa_phase10_p8_v2_0_0 complete"
