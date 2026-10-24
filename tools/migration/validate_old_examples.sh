#!/usr/bin/env bash
set -u

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SOURCE_ROOT="${SOURCE_ROOT:-$ROOT/_archive/old_examples}"
NCC_BIN="${NCC_BIN:-$ROOT/target/debug/ncc}"
DOMAIN="${1:-n}"
LIMIT="${2:-0}"
ERROR_LIMIT="${ERROR_LIMIT:-30}"
PATHS_ONLY="${PATHS_ONLY:-0}"

if [[ ! -x "$NCC_BIN" ]]; then
  printf 'error: ncc executable not found: %s\n' "$NCC_BIN" >&2
  exit 2
fi
if [[ ! -d "$SOURCE_ROOT" ]]; then
  printf 'error: source root not found: %s\n' "$SOURCE_ROOT" >&2
  exit 2
fi

case "$DOMAIN" in
  n) extension=n ;;
  nm) extension=nm ;;
  nb) extension=nb ;;
  nbb) extension=nbb ;;
  ncp) extension=ncp ;;
  *)
    printf 'usage: %s <n|nm|nb|nbb|ncp> [limit]\n' "$0" >&2
    exit 2
    ;;
esac

is_negative_fixture() {
  case "$1" in
    */negative/*)
      return 0
      ;;
    *fail*|*bad*|*invalid*|*negative*|*reject*|*error*|*forbidden*|*missing*|*unresolved*)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

run_authority() {
  local file="$1"
  case "$DOMAIN" in
    n) "$NCC_BIN" check "$file" ;;
    nm) "$NCC_BIN" machine check "$file" ;;
    nb) "$NCC_BIN" basal check "$file" ;;
    nbb) "$NCC_BIN" basal verify "$file" ;;
    ncp)
      case "${file##*/}" in
        material_advanced_contracts_ok.ncp)
          "$NCC_BIN" primitive check "$file" --target x86_64-protocol-lab
          ;;
        portio_permission_fail.ncp)
          "$NCC_BIN" primitive check "$file" \
            --target x86_64-linux-portio --environment privileged_host
          ;;
        real_hardware_missing_fail.ncp)
          "$NCC_BIN" primitive run "$file" \
            --target x86_64-linux-host-serial \
            --device /dev/nc-missing-primitive-test-device --json
          ;;
        unsafe_mmio_write_fail.ncp)
          "$NCC_BIN" primitive lower "$file" --json
          ;;
        *) "$NCC_BIN" primitive check "$file" ;;
      esac
      ;;
  esac
}

files=0
positive_pass=0
negative_pass=0
positive_fail=0
negative_unexpected_pass=0

while IFS= read -r -d '' file; do
  if (( LIMIT > 0 && files >= LIMIT )); then
    break
  fi
  ((files += 1))

  output="$(run_authority "$file" 2>&1)"
  status=$?
  if is_negative_fixture "$file"; then
    if (( status == 0 )) \
      && [[ "${file##*/}" == "real_hardware_missing_fail.ncp" ]] \
      && [[ "$output" == *'"status": "blocked"'* ]] \
      && [[ "$output" == *'"physical_hardware_certified": false'* ]]; then
      ((negative_pass += 1))
    elif (( status == 0 )); then
      ((negative_unexpected_pass += 1))
      if (( negative_unexpected_pass + positive_fail <= ERROR_LIMIT )); then
        if [[ "$PATHS_ONLY" == "1" ]]; then
          printf 'NEGATIVE_UNEXPECTED_PASS\t%s\n' "${file#"$ROOT/"}"
        else
          printf 'NEGATIVE_UNEXPECTED_PASS\t%s\n%s\n' "${file#"$ROOT/"}" "$output"
        fi
      fi
    else
      ((negative_pass += 1))
    fi
  elif (( status == 0 )); then
    ((positive_pass += 1))
  else
    ((positive_fail += 1))
    if (( negative_unexpected_pass + positive_fail <= ERROR_LIMIT )); then
      if [[ "$PATHS_ONLY" == "1" ]]; then
        printf 'POSITIVE_FAIL\t%s\n' "${file#"$ROOT/"}"
      else
        printf 'POSITIVE_FAIL\t%s\n%s\n' "${file#"$ROOT/"}" "$output"
      fi
    fi
  fi
done < <(find "$SOURCE_ROOT" -type f -name "*.$extension" -print0 | sort -z)

printf 'DOMAIN=%s FILES=%d POSITIVE_PASS=%d NEGATIVE_PASS=%d POSITIVE_FAIL=%d NEGATIVE_UNEXPECTED_PASS=%d\n' \
  "$DOMAIN" "$files" "$positive_pass" "$negative_pass" "$positive_fail" "$negative_unexpected_pass"

(( positive_fail == 0 && negative_unexpected_pass == 0 ))
