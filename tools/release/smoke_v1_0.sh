#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

OUT_DIR="${OUT_DIR:-/tmp/nc_v1_0_smoke}"
mkdir -p "$OUT_DIR"

echo "[F24] smoke check/run root fixture"
cargo run -q -p ncc-cli --bin ncc -- check examples/l2k_s8c_runtime_inspect_runtime_paths_ok.n
cargo run -q -p ncc-cli --bin ncc -- run examples/l2k_s8c_runtime_inspect_runtime_paths_ok.n

echo "[F24] smoke check machine + primitive"
cargo run -q -p ncc-cli --bin ncc -- check examples/machine/legacy_at/machine_at_strict_ok.nm
cargo run -q -p ncc-cli --bin ncc -- check examples/primitive_at/primitive_at_boot_serial_material_ok.ncp

echo "[F24] smoke artifact emission"
cargo run -q -p ncc-cli --bin ncc -- emit-elf examples/machine/legacy_at/machine_at_strict_ok.nm "$OUT_DIR/machine.o"
cargo run -q -p ncc-cli --bin ncc -- emit-coff examples/machine/legacy_at/machine_at_strict_ok.nm "$OUT_DIR/machine.obj"
cargo run -q -p ncc-cli --bin ncc -- emit-macho examples/machine/legacy_at/machine_at_strict_ok.nm "$OUT_DIR/machine_macho.o"
cargo run -q -p ncc-cli --bin ncc -- build-exe examples/linking_l9_full_ok.n "$OUT_DIR/linking.out" --cc nC.sysv64

for artifact in "$OUT_DIR/machine.o" "$OUT_DIR/machine.obj" "$OUT_DIR/machine_macho.o" "$OUT_DIR/linking.out"; do
  if [[ ! -s "$artifact" ]]; then
    echo "[F24] artifact missing or empty: $artifact" >&2
    exit 1
  fi
done

echo "[F24] smoke contract OK ($OUT_DIR)"
