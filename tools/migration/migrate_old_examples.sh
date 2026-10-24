#!/usr/bin/env bash
set -u

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MODE="${1:-audit}"
LIMIT="${2:-0}"
NCC_BIN="${NCC_BIN:-$ROOT/target/debug/ncc}"
SOURCE_ROOT="${SOURCE_ROOT:-$ROOT/_archive/old_examples}"
ERROR_LIMIT="${ERROR_LIMIT:-20}"

if [[ ! -x "$NCC_BIN" ]]; then
  printf 'error: ncc executable not found: %s\n' "$NCC_BIN" >&2
  exit 2
fi
if [[ ! -d "$SOURCE_ROOT" ]]; then
  printf 'error: source root not found: %s\n' "$SOURCE_ROOT" >&2
  exit 2
fi
if [[ "$MODE" != "audit" && "$MODE" != "format" ]]; then
  printf 'usage: %s [audit|format] [limit]\n' "$0" >&2
  exit 2
fi

count=0
canonical=0
changed=0
errors=0
while IFS= read -r -d '' file; do
  if (( LIMIT > 0 && count >= LIMIT )); then
    break
  fi
  ((count += 1))
  fixture_flag=()
  case "${file##*/}" in
    *fail*.n|*bad*.n|*invalid*.n|*negative*.n|*reject*.n|*error*.n|*forbidden*.n|*missing*.n|*unresolved*.n)
      fixture_flag=(--allow-invalid)
      ;;
  esac

  if [[ "$MODE" == "audit" ]]; then
    output="$($NCC_BIN nfmt "$file" --check --quiet "${fixture_flag[@]}" 2>&1)"
    status=$?
    if (( status == 0 )); then
      ((canonical += 1))
    elif [[ "$output" == *"formatting changes are required"* ]]; then
      ((changed += 1))
    else
      ((errors += 1))
      if (( errors <= ERROR_LIMIT )); then
        printf 'FORMAT_ERROR\t%s\n%s\n' "${file#"$ROOT/"}" "$output"
      fi
    fi
    continue
  fi

  before="$(sha256sum "$file" | cut -d' ' -f1)"
    output="$($NCC_BIN nfmt "$file" --in-place --quiet "${fixture_flag[@]}" 2>&1)"
  status=$?
  if (( status != 0 )); then
    ((errors += 1))
    if (( errors <= ERROR_LIMIT )); then
      printf 'FORMAT_ERROR\t%s\n%s\n' "${file#"$ROOT/"}" "$output"
    fi
    continue
  fi
  after="$(sha256sum "$file" | cut -d' ' -f1)"
  if [[ "$before" == "$after" ]]; then
    ((canonical += 1))
  else
    ((changed += 1))
  fi
done < <(find "$SOURCE_ROOT" -type f -name '*.n' -print0 | sort -z)

printf 'MODE=%s FILES=%d CANONICAL=%d CHANGED_OR_REQUIRED=%d ERRORS=%d\n' \
  "$MODE" "$count" "$canonical" "$changed" "$errors"
(( errors == 0 ))
