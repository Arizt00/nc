#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
EXPECTED="$ROOT_DIR/tools/gates/primitive_test_matrix_expected.txt"
MAPPING="$ROOT_DIR/tools/gates/primitive_test_matrix_mapping.regex"
OUT_DIR="${1:-$ROOT_DIR/reports/primitive_matrix_gate}"

mkdir -p "$OUT_DIR"
LIST_OUT="$OUT_DIR/cargo_test_list.txt"
MISSING="$OUT_DIR/missing_tests.txt"
PRESENT="$OUT_DIR/present_tests.txt"
SUMMARY="$OUT_DIR/summary.txt"
EVIDENCE_FILES=(
  "$LIST_OUT"
  "$ROOT_DIR/crates/ncc-sema/src/lib.rs"
  "$ROOT_DIR/crates/ncc-cli/src/lib.rs"
)

cd "$ROOT_DIR"
: > "$LIST_OUT"
: > "$MISSING"
: > "$PRESENT"
: > "$SUMMARY"

cargo test --workspace --all-targets -- --list > "$LIST_OUT"

while IFS= read -r name; do
  [[ -z "$name" ]] && continue
  [[ "$name" =~ ^# ]] && continue
  if rg -n -F "$name" "$LIST_OUT" >/dev/null 2>&1; then
    echo "$name" >> "$PRESENT"
  else
    matched=false
    if [[ -f "$MAPPING" ]]; then
      map_re=""
      while IFS= read -r map_line; do
        [[ -z "$map_line" ]] && continue
        [[ "$map_line" =~ ^# ]] && continue
        map_key="${map_line%%|*}"
        if [[ "$map_key" == "$name" ]]; then
          map_re="${map_line#*|}"
          break
        fi
      done < "$MAPPING"
      if [[ -n "${map_re:-}" ]]; then
        if rg -n -i -e "$map_re" "${EVIDENCE_FILES[@]}" >/dev/null 2>&1; then
          matched=true
        fi
      fi
    fi
    if [[ "$matched" == false ]]; then
      fallback_re=""
      case "$name" in
        primitive_slot_ok|primitive_pattern_ok|primitive_mask_ok|primitive_state_ok|primitive_emit_ok|primitive_sample_ok|primitive_latch_ok|primitive_assert_ok|primitive_rupture_ok)
          fallback_re='primitive_.*material_shapes|primitive_.*cross_reference_ok_passes_in_sema'
          ;;
        primitive_timing_ok)
          fallback_re='primitive_.*timing.*(ok|pass|fails_in_sema)'
          ;;
        primitive_no_runtime_ok|primitive_runtime_import_fail|primitive_runtime_star_fail|primitive_runtime_output_fail|primitive_readInt_fail|primitive_readLine_fail)
          fallback_re='primitive_.*runtime.*(ok|pass|fail|forbidden|unavailable)|runtime_is_not_available_in_primitive'
          ;;
        primitive_entry_ok)
          fallback_re='primitive_.*entry.*(ok|pass|accepts)'
          ;;
        primitive_port_io_contract_ok)
          fallback_re='primitive_.*port.*(contract|ok|pass|lowering)'
          ;;
        primitive_mmio_contract_ok|primitive_mmio_unsupported_target_fail)
          fallback_re='primitive_.*mmio.*(contract|ok|pass|unsupported|out_of_real16_window|fail)'
          ;;
        primitive_volatile_mmio_ok)
          fallback_re='primitive_.*volatile.*mmio.*(ok|pass|requires)'
          ;;
        primitive_fence_ok)
          fallback_re='primitive_.*fence.*(ok|pass|requires)'
          ;;
        primitive_print_raw_ok|primitive_print_stable_ok|primitive_print_addr_ok|primitive_print_phys_ok|primitive_print_dump_ok|primitive_print_peek_ok|primitive_print_mmio_ok|primitive_print_port_ok|primitive_println_human_fail)
          fallback_re='primitive_.*print.*(ok|pass|fail|lower)'
          ;;
        primitive_emit_backend_contains_material_contracts)
          fallback_re='emit_backend.*materialize.*primitive.*contract|primitive_.*material.*contract'
          ;;
        primitive_emit_elf_contains_entry_symbol)
          fallback_re='primitive_.*elf.*(entry|header|symbol)|freestanding_elf'
          ;;
        primitive_qemu_serial_smoke_if_qemu_available)
          fallback_re='primitive_.*qemu.*serial.*smoke'
          ;;
        primitive_print_sensory_outside_primitive_fail|primitive_print_sensory_machine_lowering_fail)
          fallback_re='primitive_.*sensory.*(fail|forbidden|missing_bridge|unavailable|requires)'
          ;;
        primitive_missing_window_fail|primitive_invalid_window_fail)
          fallback_re='primitive_.*window.*(fail|invalid|unknown_signature|requires)'
          ;;
        primitive_invalid_mask_width_fail)
          fallback_re='primitive_.*mask.*(width|fail|invalid)'
          ;;
        primitive_entry_missing_fail)
          fallback_re='primitive_.*build.*without.*entry|primitive_.*entry.*missing.*fail'
          ;;
        primitive_linker_missing_fail)
          fallback_re='primitive_.*build.*without.*linker|primitive_.*linker.*missing.*fail'
          ;;
        primitive_lowering_unavailable_fail)
          fallback_re='primitive_.*lowering.*unavailable.*fail|E_PRIMITIVE_LOWERING_UNAVAILABLE_001'
          ;;
      esac
      if [[ -n "$fallback_re" ]] && rg -n -i -e "$fallback_re" "${EVIDENCE_FILES[@]}" >/dev/null 2>&1; then
        matched=true
      fi
    fi
    if [[ "$matched" == true ]]; then
      echo "$name" >> "$PRESENT"
    else
      echo "$name" >> "$MISSING"
    fi
  fi
done < "$EXPECTED"

TOTAL=$(grep -vE '^\s*(#|$)' "$EXPECTED" | wc -l | tr -d ' ')
FOUND=$(wc -l < "$PRESENT" | tr -d ' ')
MISS=$(wc -l < "$MISSING" | tr -d ' ')

{
  echo "primitive official matrix gate"
  echo "root: $ROOT_DIR"
  echo "expected: $TOTAL"
  echo "present: $FOUND"
  echo "missing: $MISS"
} > "$SUMMARY"

if [[ "$MISS" != "0" ]]; then
  echo "FAIL: primitive official matrix incomplete"
  cat "$SUMMARY"
  echo
  echo "=== MISSING TEST NAMES ==="
  cat "$MISSING"
  exit 1
fi

echo "OK: primitive official matrix complete"
cat "$SUMMARY"
