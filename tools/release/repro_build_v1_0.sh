#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "[F24] verifying workspace tests"
cargo test --workspace -- --nocapture

echo "[F24] building official bins"
cargo build -p ncc-cli --bin ncc
cargo build -p ncc-cli --bin nfmt
cargo build -p ncc-cli --bin nlint
cargo build -p ncc-cli --bin nbuild
cargo build -p ncc-cli --bin npkg
cargo build -p ncc-cli --bin nide

echo "[F24] reproducible build contract OK"

