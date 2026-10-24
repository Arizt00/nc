#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ALLOWLIST="$ROOT_DIR/tools/gates/primitive_debt_allowlist.regex"
OUT_DIR="${1:-$ROOT_DIR/reports/primitive_debt_gate}"
TOKENS='\bTODO\b|\bFIXME\b|\bHACK\b|unimplemented!\(|todo!\('

mkdir -p "$OUT_DIR"
RAW="$OUT_DIR/raw_matches.txt"
NEW="$OUT_DIR/new_matches.txt"
SUMMARY="$OUT_DIR/summary.txt"

cd "$ROOT_DIR"

: > "$RAW"
: > "$NEW"
: > "$SUMMARY"

# Scan scope: primitive closure critical surfaces only.
SCAN_PATHS=(
  "examples/primitive"
  "crates/ncc-sema/src/lib.rs"
  "crates/ncc-cli/src/lib.rs"
)

EXISTING_PATHS=()
for p in "${SCAN_PATHS[@]}"; do
  [[ -e "$p" ]] && EXISTING_PATHS+=("$p")
done

if [[ ${#EXISTING_PATHS[@]} -eq 0 ]]; then
  echo "ERROR: no scan paths found for primitive debt gate" >&2
  exit 2
fi

rg -n "$TOKENS" "${EXISTING_PATHS[@]}" \
  --glob '!crates/ncc-cli/src/lib.rs.bak-version' \
  --glob '!**/*.md' \
  --glob '!**/*.txt' \
  --glob '!**/*.regex' \
  --glob '!**/*.bak-version' \
  --glob '!docs/archive/**' \
  --glob '!reports/**' \
  > "$RAW" || true

if [[ ! -f "$ALLOWLIST" ]]; then
  echo "ERROR: allowlist not found: $ALLOWLIST" >&2
  exit 2
fi

while IFS= read -r line; do
  [[ -z "$line" ]] && continue
  allowed=false
  while IFS= read -r re; do
    [[ -z "$re" ]] && continue
    [[ "$re" =~ ^# ]] && continue
    if [[ "$line" =~ $re ]]; then
      allowed=true
      break
    fi
  done < "$ALLOWLIST"
  if [[ "$allowed" == false ]]; then
    echo "$line" >> "$NEW"
  fi
done < "$RAW"

{
  echo "primitive strict debt gate"
  echo "root: $ROOT_DIR"
  echo "raw_matches: $(wc -l < "$RAW" | tr -d ' ')"
  echo "new_matches: $(wc -l < "$NEW" | tr -d ' ')"
} > "$SUMMARY"

if [[ -s "$NEW" ]]; then
  echo "FAIL: new debt-token matches detected"
  cat "$SUMMARY"
  echo
  echo "=== NEW MATCHES ==="
  cat "$NEW"
  exit 1
fi

echo "OK: no new debt-token matches"
cat "$SUMMARY"
