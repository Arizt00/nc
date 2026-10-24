#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ncc="${1:-$repo_root/target/debug/ncc}"
examples_root="${2:-$repo_root/examples}"

if [[ ! -x "$ncc" ]]; then
  ncc="$repo_root/target/release/ncc"
fi
if [[ ! -x "$ncc" ]]; then
  printf 'ncc executable not found: %s\n' "$ncc" >&2
  exit 2
fi

total=0
positive_ok=0
positive_failed=0
negative_rejected=0
negative_accepted=0
projects_ok=0
projects_failed=0
failures=()
jobs="${NCC_EXAMPLE_JOBS:-1}"
if [[ ! "$jobs" =~ ^[1-9][0-9]*$ ]]; then
  printf 'NCC_EXAMPLE_JOBS must be a positive integer, got: %s\n' "$jobs" >&2
  exit 2
fi

result_dir="$(mktemp -d)"
cleanup() {
  rm -rf -- "$result_dir"
}
trap cleanup EXIT

run_case() {
  local file="$1"
  local relative="$2"
  local negative="$3"
  local project="$4"
  local result_file="$5"
  local output summary

  if (( project )); then
    if output="$($ncc project check "$(dirname "$file")" 2>&1)"; then
      printf 'PROJECT_OK\t%s\t\n' "$relative" > "$result_file"
    else
      summary="$(printf '%s\n' "$output" | head -n 3 | tr '\n\t' '||')"
      printf 'PROJECT_FAIL\t%s\t%s\n' "$relative" "$summary" > "$result_file"
    fi
    return
  fi

  if output="$($ncc check "$file" 2>&1)"; then
    if (( negative )); then
      printf 'NEGATIVE_ACCEPTED\t%s\t\n' "$relative" > "$result_file"
    else
      printf 'POSITIVE_OK\t%s\t\n' "$relative" > "$result_file"
    fi
  elif (( negative )); then
    printf 'NEGATIVE_REJECTED\t%s\t\n' "$relative" > "$result_file"
  else
    summary="$(printf '%s\n' "$output" | head -n 3 | tr '\n\t' '||')"
    printf 'POSITIVE_FAIL\t%s\t%s\n' "$relative" "$summary" > "$result_file"
  fi
}

active=0

while IFS= read -r -d '' file; do
  ((total += 1))
  relative="${file#"$examples_root"/}"
  base="$(basename "$file" .n)"
  negative=0
  if [[ "/$relative" == */negative/* || "$base" =~ (_fail|_error|_bad_type)$ ]]; then
    negative=1
  fi
  project=0
  if (( ! negative )) && grep -Eq 'use[[:space:]]*(::|<)[[:space:]]*project' "$file"; then
    project=1
  fi

  result_file="$result_dir/$(printf '%06d' "$total").result"
  run_case "$file" "$relative" "$negative" "$project" "$result_file" &
  ((active += 1))
  if (( active >= jobs )); then
    wait -n || true
    active=$((active - 1))
  fi
done < <(find "$examples_root" -type f -name '*.n' -not -path '*/.ncc/*' -print0 | sort -z)

while (( active > 0 )); do
  wait -n || true
  active=$((active - 1))
done

for result_file in "$result_dir"/*.result; do
  IFS=$'\t' read -r kind relative detail < "$result_file"
  case "$kind" in
    POSITIVE_OK) ((positive_ok += 1)) ;;
    POSITIVE_FAIL)
      ((positive_failed += 1))
      failures+=("POSITIVE_FAIL $relative :: $detail")
      ;;
    NEGATIVE_REJECTED) ((negative_rejected += 1)) ;;
    NEGATIVE_ACCEPTED)
      ((negative_accepted += 1))
      failures+=("NEGATIVE_ACCEPTED $relative")
      ;;
    PROJECT_OK) ((projects_ok += 1)) ;;
    PROJECT_FAIL)
      ((projects_failed += 1))
      failures+=("PROJECT_FAIL $relative :: $detail")
      ;;
  esac
done

printf 'total=%d\npositive_ok=%d\npositive_failed=%d\nnegative_rejected=%d\nnegative_accepted=%d\nprojects_ok=%d\nprojects_failed=%d\n' \
  "$total" "$positive_ok" "$positive_failed" "$negative_rejected" "$negative_accepted" "$projects_ok" "$projects_failed"

if ((${#failures[@]})); then
  printf '%s\n' '--- failures ---'
  printf '%s\n' "${failures[@]}"
fi

if ((positive_failed || negative_accepted || projects_failed)); then
  exit 1
fi
