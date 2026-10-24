#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

"${NCC_NATIVE_TOOLS:-ncc-tools-v43}" release-imports --root "$ROOT_DIR"
