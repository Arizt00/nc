#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
EXPECTED_CODES="$ROOT_DIR/tools/gates/primitive_expected_codes.txt"
OUT_DIR="${1:-$ROOT_DIR/reports/primitive_diagnostic_gate}"

mkdir -p "$OUT_DIR"
FOUND="$OUT_DIR/found_codes.txt"
MISSING="$OUT_DIR/missing_codes.txt"
SUMMARY="$OUT_DIR/summary.txt"

cd "$ROOT_DIR"

if [[ ! -f "$EXPECTED_CODES" ]]; then
  echo "ERROR: expected codes file not found: $EXPECTED_CODES" >&2
  exit 2
fi

touch "$FOUND" "$MISSING"

while IFS= read -r code; do
  [[ -z "$code" ]] && continue
  [[ "$code" =~ ^# ]] && continue
  if rg -n -F "$code" crates/ncc-sema/src crates/ncc-cli/src >/dev/null 2>&1; then
    echo "$code" >> "$FOUND"
  else
    echo "$code" >> "$MISSING"
  fi
done < "$EXPECTED_CODES"

TOTAL=$(grep -vE '^\s*(#|$)' "$EXPECTED_CODES" | wc -l | tr -d ' ')
FOUND_N=$(wc -l < "$FOUND" | tr -d ' ')
MISSING_N=$(wc -l < "$MISSING" | tr -d ' ')

{
  echo "primitive diagnostic uniformity gate"
  echo "root: $ROOT_DIR"
  echo "expected_codes: $TOTAL"
  echo "found_codes: $FOUND_N"
  echo "missing_codes: $MISSING_N"
} > "$SUMMARY"

if [[ "$MISSING_N" != "0" ]]; then
  echo "FAIL: missing official primitive diagnostic codes"
  cat "$SUMMARY"
  echo
  echo "=== MISSING CODES ==="
  cat "$MISSING"
  exit 1
fi

echo "OK: all official primitive diagnostic codes are present in sema/cli"
cat "$SUMMARY"
