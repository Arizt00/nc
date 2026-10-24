#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "[PRIMITIVE-RELEASE] material gate tests"
cargo test -p ncc-cli release_primitive_material_gate_tests -- --nocapture

echo "[PRIMITIVE-RELEASE] packaging + qemu evidence verifier"
suffix=""
case "${OS:-}:$(uname -s)" in
  Windows_NT:*|*:MINGW*|*:MSYS*) suffix=".exe" ;;
esac
certification="${NCC_CERTIFICATION_BIN:-$ROOT_DIR/target/native-tools-v43/bin/ncc-certification-v43$suffix}"
if [[ ! -x "$certification" ]]; then
  echo "Native certification family is unavailable: $certification" >&2
  echo "Build ncc-certification-v43 with the retained Stage0 and bootstrap/native-tools-v43.json." >&2
  exit 2
fi
"$certification" primitive-release-evidence --repo "$ROOT_DIR"

echo "[PRIMITIVE-RELEASE] OK"
