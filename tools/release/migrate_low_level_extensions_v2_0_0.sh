#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

REPORT_W1="docs/status/F10_P1_low_level_extension_migration_wave1_report.json"
REPORT_W2="docs/status/F10_P1_low_level_extension_migration_wave2_report.json"
REPORT_W3="docs/status/F10_P1_low_level_extension_migration_wave3_report.json"
REPORT_W4="docs/status/F10_P1_low_level_extension_migration_wave4_report.json"

echo "[F10-P1] low-level extension migration wave1"
"${NCC_NATIVE_TOOLS:-ncc-tools-v43}" migrate-low-level \
  --root . \
  --wave wave1 \
  --apply \
  --report "$REPORT_W1"

echo "[F10-P1] report: $REPORT_W1"

echo "[F10-P1] low-level extension migration wave2"
"${NCC_NATIVE_TOOLS:-ncc-tools-v43}" migrate-low-level \
  --root . \
  --wave wave2 \
  --apply \
  --report "$REPORT_W2"

echo "[F10-P1] report: $REPORT_W2"

echo "[F10-P1] low-level extension migration wave3"
"${NCC_NATIVE_TOOLS:-ncc-tools-v43}" migrate-low-level \
  --root . \
  --wave wave3 \
  --apply \
  --report "$REPORT_W3"

echo "[F10-P1] report: $REPORT_W3"

echo "[F10-P1] low-level extension migration wave4 (strict closure)"
"${NCC_NATIVE_TOOLS:-ncc-tools-v43}" migrate-low-level \
  --root . \
  --wave wave4 \
  --apply \
  --report "$REPORT_W4" \
  --require-strict-ready

echo "[F10-P1] report: $REPORT_W4"

echo "[F10-P1] strict validation (official extensions)"
cargo run -q -p ncc-cli --bin ncc -- nclsp validate examples/root_at/root_at_minimal_ok.nb --profile expert --migration-stage strict --json >/dev/null
cargo run -q -p ncc-cli --bin ncc -- nclsp validate examples/machine/legacy_at/machine_at_strict_ok.nm --profile expert --migration-stage strict --json >/dev/null
cargo run -q -p ncc-cli --bin ncc -- nclsp validate examples/primitive_at/primitive_at_boot_serial_material_ok.ncp --profile expert --migration-stage strict --json >/dev/null
cargo run -q -p ncc-cli --bin ncc -- nclsp validate examples/primitive_at/primitive_at_anchor_inline_clause_ok.machine.nm --profile expert --migration-stage strict --json >/dev/null
cargo run -q -p ncc-cli --bin ncc -- nclsp validate examples/primitive_at/primitive_at_boot_serial_short_header_ok.machine.nm --profile expert --migration-stage strict --json >/dev/null
cargo run -q -p ncc-cli --bin ncc -- nclsp validate examples/user_v1_0_1/ejemplo9_control_soberano_raiz.nb --profile expert --migration-stage strict --json >/dev/null
cargo run -q -p ncc-cli --bin ncc -- nclsp validate examples/user_batch_20260501/ejemplo10_hash_mixer_machine.nm --profile expert --migration-stage strict --json >/dev/null
cargo run -q -p ncc-cli --bin ncc -- nclsp validate examples/user_v1_0_1/ejemplo11_boot_fabric.ncp --profile expert --migration-stage strict --json >/dev/null
cargo run -q -p ncc-cli --bin ncc -- nclsp validate examples/user_suite_v1_0_1/04_primitive_modes.norm.ncp --profile expert --migration-stage strict --json >/dev/null
cargo run -q -p ncc-cli --bin ncc -- nclsp validate examples/user_suite_v1_0_1/04_primitive_modes.norm.machine.nm --profile expert --migration-stage strict --json >/dev/null

echo "[F10-P1] migration wave1+wave2+wave3+wave4 OK"
