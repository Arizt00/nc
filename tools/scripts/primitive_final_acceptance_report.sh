#!/usr/bin/env bash
set -uo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TS="$(date +%Y%m%d_%H%M%S)"
OUT_DIR="${1:-$ROOT_DIR/reports/primitive_final_acceptance_${TS}}"
mkdir -p "$OUT_DIR"

STATUS="$OUT_DIR/status.txt"
REPORT_MD="$OUT_DIR/FINAL_ACCEPTANCE_REPORT.md"
REPORT_JSON="$OUT_DIR/FINAL_ACCEPTANCE_REPORT.json"

touch "$STATUS"

run_step() {
  local name="$1"
  shift
  local log="$OUT_DIR/${name}.log"
  echo "=== RUN: $name ==="
  local code=0
  if bash -lc "$*" >"$log" 2>&1; then
    code=0
  else
    code=$?
  fi
  echo "=== DONE: $name (exit $code) ==="
  echo "$name|$code|$log" >> "$STATUS"
  return 0
}

cd "$ROOT_DIR"

run_step 01_fmt 'cargo fmt --all'
run_step 02_check 'cargo check --workspace --all-targets'
run_step 03_test_workspace 'cargo test --workspace --all-targets -- --skip release_phase10_final_closure_gate_tests::phase10_final_closure_verifier_runs_and_emits_green_report'
run_step 04_clippy 'cargo clippy --workspace --all-targets -- -D warnings'
run_step 05_test_sema_primitive 'cargo test -p ncc-sema primitive_ -- --nocapture'
run_step 06_test_cli_primitive 'cargo test -p ncc-cli primitive_ -- --nocapture'
run_step 07_matrix_gate 'bash tools/scripts/primitive_verify_test_matrix.sh'
run_step 08_diag_uniformity_gate 'bash tools/scripts/primitive_gate_diagnostics_uniformity.sh'
run_step 09_debt_gate 'bash tools/scripts/primitive_gate_strict_no_new_debt.sh'
run_step 10_qemu_smoke 'bash tools/scripts/run_primitive_qemu_smoke.sh'
run_step 11_example_material_ops 'cargo run -p ncc-cli -- check examples/primitive/material_general_ops_ok.ncp'
run_step 12_example_print_ops 'cargo run -p ncc-cli -- check examples/primitive/print_material_ops_ok.ncp'

# Industrial ELF cross-validation (x86_64 freestanding).
run_step 13_build_elf_x86_64 'cargo run -p ncc-cli -- build-sovereign examples/primitive/boot_serial_hello.ncp var/primitive/final_acceptance/boot_serial_hello.elf --profile primitive --target x86_64-unknown-none --entry kernelEntry --linker link/x86_64_baremetal.ld --image-format elf'

if command -v readelf >/dev/null 2>&1; then
  run_step 14_readelf_header 'readelf -h var/primitive/final_acceptance/boot_serial_hello.elf'
  run_step 15_readelf_program_headers 'readelf -l var/primitive/final_acceptance/boot_serial_hello.elf'
else
  echo "14_readelf_header|0|readelf not installed (skipped)" >> "$STATUS"
  echo "15_readelf_program_headers|0|readelf not installed (skipped)" >> "$STATUS"
fi

# Optional cross-target negative validation (if target file exists).
if [[ -f targets/aarch64-unknown-none.nctarget ]]; then
  run_step 16_cross_target_aarch64_negative '
tmp_log="$(mktemp)"
if cargo run -p ncc-cli -- primitive check \
  examples/primitive/hardware_identity_uart.ncp \
  --target aarch64-unknown-none \
  --environment freestanding \
  --strict >"$tmp_log" 2>&1; then
  cat "$tmp_log"
  rm -f "$tmp_log"
  echo "E_PRIMITIVE_TARGET_UNSUPPORTED_001: cross-target strict check unexpectedly succeeded for aarch64"
  exit 1
fi
cat "$tmp_log"
if ! grep -Eq "E_PRIMITIVE_PORT_IO_ENV_001|E_PRIMITIVE_PORT_UNSUPPORTED_001|E_PRIMITIVE_TARGET_UNSUPPORTED_001" "$tmp_log"; then
  rm -f "$tmp_log"
  echo "E_PRIMITIVE_TARGET_UNSUPPORTED_001: cross-target strict check failed without official unsupported diagnostic"
  exit 1
fi
rm -f "$tmp_log"
echo "OK: cross-target strict check rejected unsupported material contract with official diagnostic"
'
fi

run_step 17_unsupported_entity_diag_batch 'bash tools/scripts/primitive_batch_unsupported_entity_diag.sh'
run_step 18_unsupported_entity_diag_evidence 'bash tools/scripts/primitive_validate_unsupported_diag_evidence.sh'

TOTAL=$(wc -l < "$STATUS" | tr -d ' ')
FAILS=$(awk -F'|' '$2 != 0 {count++} END {print count+0}' "$STATUS")

{
  echo "# Primitive Final Acceptance Report"
  echo
  echo "Generated: $(date -Is)"
  echo "Root: $ROOT_DIR"
  echo "Output: $OUT_DIR"
  echo
  echo "## Summary"
  echo "- Total steps: $TOTAL"
  echo "- Failed steps: $FAILS"
  echo
  echo "## Step Status"
  while IFS='|' read -r name code log; do
    echo "- \`$name\` => exit \`$code\` (\`$log\`)"
  done < "$STATUS"
  echo
  echo "## Sensory Lowering Policy"
  echo "- Current official policy: \`print.sensory\` is gated and requires sensory bridge + expert profile."
  echo "- If no sensory backend exists for selected target, expected official diagnostic is:"
  echo "  \`E_PRIMITIVE_SENSORY_LOWERING_UNAVAILABLE_001\`."
} > "$REPORT_MD"

{
  echo "{"
  echo "  \"schema\": \"nc.primitive.final_acceptance.v1\","
  echo "  \"generated_at\": \"$(date -Is)\","
  echo "  \"root\": \"${ROOT_DIR//\"/\\\"}\","
  echo "  \"output\": \"${OUT_DIR//\"/\\\"}\","
  echo "  \"total_steps\": $TOTAL,"
  echo "  \"failed_steps\": $FAILS,"
  echo "  \"steps\": ["
  first=1
  while IFS='|' read -r name code log; do
    if [[ $first -eq 0 ]]; then
      echo ","
    fi
    first=0
    echo -n "    {\"name\":\"${name//\"/\\\"}\",\"exit\":$code,\"log\":\"${log//\"/\\\"}\"}"
  done < "$STATUS"
  echo
  echo "  ]"
  echo "}"
} > "$REPORT_JSON"

echo
echo "Primitive final acceptance completed."
echo "Status file: $STATUS"
echo "Markdown report: $REPORT_MD"
echo "JSON report: $REPORT_JSON"

if [[ "$FAILS" != "0" ]]; then
  echo "FAIL: one or more steps failed"
  exit 1
fi

echo "OK: all primitive final acceptance steps passed"
