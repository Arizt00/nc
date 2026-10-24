#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "[F10-OPS] verify external publication / prod-key runbook / clean-machine / release notes"
"${NCC_NATIVE_TOOLS:-ncc-tools-v43}" dist-external-ops --root "$ROOT_DIR" \
  --report "$ROOT_DIR/docs/status/F10_external_ops_report.json"

echo "[F10-OPS] gate OK"
