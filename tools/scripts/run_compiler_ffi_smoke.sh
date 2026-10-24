#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
mkdir -p reports/compiler
cargo run -p ncc-cli -- compiler ffi-smoke --json > reports/compiler/compiler_ffi_smoke.json
printf 'OK: compiler ffi smoke report %s\n' "$ROOT/reports/compiler/compiler_ffi_smoke.json"
