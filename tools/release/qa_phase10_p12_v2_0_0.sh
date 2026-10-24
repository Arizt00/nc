#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

export NCC_NATIVE_TOOLS="${NCC_NATIVE_TOOLS:-ncc-ide-v43}"
if ! command -v "$NCC_NATIVE_TOOLS" >/dev/null 2>&1; then
    echo "Missing native IDE family: $NCC_NATIVE_TOOLS (build family ncc-ide-v43)" >&2
    exit 2
fi

echo "[F10-P12] file association + global execution gate"
cargo test -p ncc-cli release_phase10_file_association_global_exec_gate_tests -- --nocapture
"$NCC_NATIVE_TOOLS" ide43-association --root "$ROOT_DIR" --router "$NCC_NATIVE_TOOLS"

echo "[F10-P12] OK"
