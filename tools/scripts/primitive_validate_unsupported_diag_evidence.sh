#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
JSON_REPORT="${1:-$ROOT_DIR/var/primitive/unsupported_diag_batch/unsupported_diag_batch_report.json}"
MD_REPORT="${2:-$ROOT_DIR/var/primitive/unsupported_diag_batch/unsupported_diag_batch_report.md}"

fail() {
  echo "FAIL: $1" >&2
  exit 1
}

test -s "$JSON_REPORT" || fail "missing JSON report: $JSON_REPORT"
test -s "$MD_REPORT" || fail "missing Markdown report: $MD_REPORT"

rg -n '"schema":[[:space:]]*"nc\.primitive\.unsupported_diag_batch\.v1"' "$JSON_REPORT" >/dev/null \
  || fail "JSON report schema mismatch"
rg -n '"status":[[:space:]]*"ok"' "$JSON_REPORT" >/dev/null \
  || fail "JSON report status is not ok"
rg -n '^# Primitive Unsupported Diagnostics Batch Report$' "$MD_REPORT" >/dev/null \
  || fail "Markdown report title mismatch"

total_cases="$(sed -n 's/.*"total_cases":[[:space:]]*\([0-9][0-9]*\).*/\1/p' "$JSON_REPORT" | head -n 1)"
[[ -n "$total_cases" ]] || fail "JSON report does not expose total_cases"

case_count="$(rg -o '"case":"' "$JSON_REPORT" | wc -l | tr -d ' ')"
primary_count="$(rg -o '"matched_primary":true' "$JSON_REPORT" | wc -l | tr -d ' ')"
secondary_count="$(rg -o '"matched_secondary":true' "$JSON_REPORT" | wc -l | tr -d ' ')"

[[ "$case_count" == "$total_cases" ]] || fail "case count ($case_count) != total_cases ($total_cases)"
[[ "$primary_count" == "$total_cases" ]] || fail "matched_primary true count ($primary_count) != total_cases ($total_cases)"
[[ "$secondary_count" == "$total_cases" ]] || fail "matched_secondary true count ($secondary_count) != total_cases ($total_cases)"

required_cases=(
  "power_model_unsupported"
  "thermal_model_unsupported"
  "cache_model_unsupported"
  "tlb_model_unsupported"
  "pipeline_model_unsupported"
  "firmware_model_unsupported"
  "bootrom_model_unsupported"
  "microcode_opaque_unsupported"
  "port_io_cross_target_unsupported"
  "protocol_usb_unsupported"
)

for case_name in "${required_cases[@]}"; do
  rg -F "\"case\":\"${case_name}\"" "$JSON_REPORT" >/dev/null \
    || fail "missing case in JSON report: ${case_name}"
  rg -F -- "- ${case_name}" "$MD_REPORT" >/dev/null \
    || fail "missing case in Markdown report: ${case_name}"
done

while IFS= read -r log_path; do
  [[ -n "$log_path" ]] || continue
  test -s "$log_path" || fail "missing case log referenced by JSON report: $log_path"
done < <(rg -o '"log":"[^"]+"' "$JSON_REPORT" | sed 's/^"log":"//; s/"$//')

echo "OK: primitive unsupported diagnostics evidence validated"
echo "json: $JSON_REPORT"
echo "md: $MD_REPORT"
echo "total_cases: $total_cases"
