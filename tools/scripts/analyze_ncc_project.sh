#!/usr/bin/env bash
set -uo pipefail

usage() {
  cat <<'EOF'
Usage:
  tools/scripts/analyze_ncc_project.sh [quick|full|machine]

Modes:
  quick    Inventory, static debt scan, cargo fmt/check, and focused ncc smoke tests.
  full     quick + full workspace tests and a wider machine fixture matrix.
  machine  Focus on [@machine] / .nm fixture continuity and typed IR smoke tests.

Environment knobs:
  NCC_ANALYZE_REPORT_DIR=reports/ncc_project_analysis
  NCC_ANALYZE_MACHINE_LIMIT=12        # quick/machine default; full default is 120
  NCC_ANALYZE_RG_LIMIT=500
  NCC_ANALYZE_CLIPPY=1                # opt-in because it can be noisy/slow here
EOF
}

MODE="${1:-quick}"
case "$MODE" in
  -h|--help|help)
    usage
    exit 0
    ;;
  quick|full|machine)
    ;;
  *)
    echo "ERROR: unknown mode '$MODE'" >&2
    usage >&2
    exit 2
    ;;
esac

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd -- "$SCRIPT_DIR/../.." && pwd)"

if [ ! -f "$ROOT/Cargo.toml" ]; then
  ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel 2>/dev/null || pwd)"
fi

cd "$ROOT" || exit 1

if [ ! -f Cargo.toml ] || [ ! -d crates/ncc-cli ]; then
  echo "ERROR: this does not look like the ncc workspace root: $ROOT" >&2
  exit 1
fi

TS="$(date +%Y%m%d_%H%M%S)"
REPORT_DIR="${NCC_ANALYZE_REPORT_DIR:-reports/ncc_project_analysis}"
mkdir -p "$REPORT_DIR"
OUT="$REPORT_DIR/ncc_project_analysis_${MODE}_${TS}.md"

if [ -z "${NCC_ANALYZE_MACHINE_LIMIT:-}" ]; then
  if [ "$MODE" = "full" ]; then
    export NCC_ANALYZE_MACHINE_LIMIT=120
  else
    export NCC_ANALYZE_MACHINE_LIMIT=12
  fi
fi

export NCC_ANALYZE_RG_LIMIT="${NCC_ANALYZE_RG_LIMIT:-500}"
export NCC_ANALYZE_NCC_BIN="${NCC_ANALYZE_NCC_BIN:-$ROOT/target/debug/ncc}"

FAILURES=()

exec > >(tee "$OUT") 2>&1

section() {
  echo
  echo "## $1"
  echo
}

record_failure() {
  FAILURES+=("$1")
}

print_cmd() {
  printf '$'
  printf ' %q' "$@"
  echo
}

run_cmd() {
  local required="$1"
  shift
  local title="$1"
  shift

  section "$title"
  echo '```bash'
  print_cmd "$@"
  echo '```'
  echo
  echo '```txt'
  "$@"
  local code=$?
  echo
  echo "[exit-code: $code]"
  echo '```'

  if [ "$required" = "required" ] && [ "$code" -ne 0 ]; then
    record_failure "$title (exit $code)"
  fi
}

run_shell() {
  local required="$1"
  shift
  local title="$1"
  shift
  local script="$1"
  local prelude
  prelude='
ncc_cmd() {
  if [ -n "${NCC_ANALYZE_NCC_BIN:-}" ] && [ -x "$NCC_ANALYZE_NCC_BIN" ]; then
    "$NCC_ANALYZE_NCC_BIN" "$@"
  else
    cargo run -q -p ncc-cli -- "$@"
  fi
}
'

  section "$title"
  echo '```bash'
  echo "$script"
  echo '```'
  echo
  echo '```txt'
  bash -lc "$prelude
$script"
  local code=$?
  echo
  echo "[exit-code: $code]"
  echo '```'

  if [ "$required" = "required" ] && [ "$code" -ne 0 ]; then
    record_failure "$title (exit $code)"
  fi
}

emit_skip() {
  section "$1"
  echo '```txt'
  echo "$2"
  echo '```'
}

cat <<EOF
# ncc Project Analysis

- Date: $(date -Is)
- Workspace: $(pwd)
- Mode: $MODE
- Report: $OUT
- ncc binary: ${NCC_ANALYZE_NCC_BIN}

This report is meant to respect the current nC rule: parser, AST, sema, IR,
tests, CLI, and docs should be analyzed together instead of closing work by
syntax alone.
EOF

run_shell optional "0. Toolchain and host" '
set +e
for tool in bash git rustc cargo rustfmt clippy-driver rg find awk sed grep wc; do
  if [ "$tool" = "rg" ]; then
    if command -v rg >/dev/null 2>&1 && rg --version >/dev/null 2>&1; then
      printf "%-16s %s\n" "$tool" "$(command -v "$tool")"
    else
      printf "%-16s MISSING_OR_NOT_EXECUTABLE\n" "$tool"
    fi
  elif command -v "$tool" >/dev/null 2>&1; then
    printf "%-16s %s\n" "$tool" "$(command -v "$tool")"
  else
    printf "%-16s MISSING\n" "$tool"
  fi
done
echo
uname -a || true
whoami || true
pwd || true
rustc --version || true
cargo --version || true
'

run_shell optional "1. Git status and local delta" '
set +e
git status --short --branch
echo
git diff --stat
echo
git diff --name-status
'

run_cmd required "2. Cargo metadata" cargo metadata --no-deps --format-version 1

run_shell optional "3. Workspace shape" '
set +e
echo "== top-level =="
find . -maxdepth 2 \
  -path "./.git" -prune -o \
  -path "./target" -prune -o \
  -path "./node_modules" -prune -o \
  -type d -print | sort
echo
echo "== crates =="
find crates -maxdepth 2 -name Cargo.toml -print | sort | xargs -r -n1 dirname
echo
echo "== ncc-cli bins =="
sed -n "/^\\[\\[bin\\]\\]/,/^$/p" crates/ncc-cli/Cargo.toml
'

run_shell optional "4. File inventory by extension" '
set +e
if command -v rg >/dev/null 2>&1 && rg --version >/dev/null 2>&1; then
  rg --files \
    -g "!target/**" \
    -g "!.git/**" \
    -g "!node_modules/**" \
    -g "!.idea/**" \
    -g "!reports/**"
else
  find . \
    -path "./target" -prune -o \
    -path "./.git" -prune -o \
    -path "./node_modules" -prune -o \
    -path "./.idea" -prune -o \
    -path "./reports" -prune -o \
    -type f -print
fi | awk '"'"'
function extension(path, base, n, a) {
  n = split(path, a, "/")
  base = a[n]
  if (base !~ /\./) return "[no_ext]"
  sub(/^.*\./, ".", base)
  return base
}
{ count[extension($0)]++; total++ }
END {
  printf "total_files %d\n", total
  for (ext in count) printf "%8d %s\n", count[ext], ext
}
'"'"' | sort -nr
'

run_shell optional "5. Approximate source lines" '
set +e
find . \
  -path "./target" -prune -o \
  -path "./.git" -prune -o \
  -path "./node_modules" -prune -o \
  -path "./.idea" -prune -o \
  -path "./reports" -prune -o \
  -type f \( \
    -name "*.rs" -o -name "*.n" -o -name "*.nm" -o -name "*.nb" -o -name "*.ncp" -o \
    -name "*.md" -o -name "*.toml" -o -name "*.json" -o -name "*.sh" -o -name "*.py" \
  \) -print0 \
| xargs -0 wc -l 2>/dev/null \
| sort -nr \
| sed -n "1,120p"
'

if [ "$MODE" != "machine" ]; then
  run_cmd required "6. Cargo fmt check" cargo fmt --all -- --check
  run_cmd required "7. Cargo check workspace" cargo check --workspace --all-targets

  if [ "$MODE" = "full" ]; then
    run_cmd required "8. Cargo test workspace" cargo test --workspace --all-targets -- --nocapture
  else
    emit_skip "8. Cargo test workspace" "Skipped in quick mode. Run: tools/scripts/analyze_ncc_project.sh full"
  fi

  if [ "${NCC_ANALYZE_CLIPPY:-0}" = "1" ]; then
    run_cmd required "9. Cargo clippy workspace" cargo clippy --workspace --all-targets -- -D warnings
  else
    emit_skip "9. Cargo clippy workspace" "Skipped by default. Run with NCC_ANALYZE_CLIPPY=1 to include it."
  fi
else
  emit_skip "6-9. Cargo workspace gates" "Skipped in machine mode; use quick/full for workspace-wide cargo gates."
fi

run_cmd required "10. Build ncc CLI once" cargo build -q -p ncc-cli --bin ncc

run_shell required "11. ncc CLI help and doctors" '
set -u
ncc_cmd --help
echo
ncc_cmd compiler doctor
echo
ncc_cmd compiler self-check
echo
ncc_cmd runtime doctor
echo
ncc_cmd primitive doctor
echo
ncc_cmd basal doctor
'

run_shell required "12. [@machine] official smoke: .n surface to IR" '
set -u
f="examples/machine_at/machine_at_nop_wait_halt_ok.n"
test -f "$f"
ncc_cmd check "$f"
echo
ncc_cmd emit-ir "$f"
'

run_shell required "13. [@machine] official smoke: .nm surface to IR" '
set -u
f="examples/machine_at/machine_at_nop_wait_halt_ok.nm"
test -f "$f"
ncc_cmd check "$f"
echo
ncc_cmd emit-ir "$f"
'

run_shell required "14. Native machine CLI smoke" '
set -u
f="examples/machine/native/x86_reg_reg_phase1.nm"
test -f "$f"
ncc_cmd machine check "$f"
echo
ncc_cmd machine emit-ir "$f"
echo
ncc_cmd machine report "$f" --json
'

run_shell required "15. [@machine] surface fixture matrix" '
set -u
limit="${NCC_ANALYZE_MACHINE_LIMIT:-12}"
echo "limit=$limit per polarity"
failures=0

echo
echo "== expected OK machine_at fixtures =="
count=0
while IFS= read -r f; do
  count=$((count + 1))
  printf "[OK:%03d] %s\n" "$count" "$f"
  if ! ncc_cmd check "$f" >/tmp/ncc_machine_ok_stdout.txt 2>/tmp/ncc_machine_ok_stderr.txt; then
    echo "UNEXPECTED FAIL: $f"
    cat /tmp/ncc_machine_ok_stdout.txt
    cat /tmp/ncc_machine_ok_stderr.txt
    failures=$((failures + 1))
  fi
done < <(find examples/machine_at -type f \( -name "*_ok.n" -o -name "*_ok.nm" \) | sort | head -n "$limit")

echo
echo "== expected FAIL machine_at fixtures =="
count=0
while IFS= read -r f; do
  count=$((count + 1))
  printf "[FAIL:%03d] %s\n" "$count" "$f"
  if ncc_cmd check "$f" >/tmp/ncc_machine_fail_stdout.txt 2>/tmp/ncc_machine_fail_stderr.txt; then
    echo "UNEXPECTED PASS: $f"
    cat /tmp/ncc_machine_fail_stdout.txt
    failures=$((failures + 1))
  fi
done < <(find examples/machine_at -type f \( -name "*_fail.n" -o -name "*_fail.nm" \) | sort | head -n "$limit")

rm -f /tmp/ncc_machine_ok_stdout.txt /tmp/ncc_machine_ok_stderr.txt /tmp/ncc_machine_fail_stdout.txt /tmp/ncc_machine_fail_stderr.txt
echo
echo "matrix_failures=$failures"
test "$failures" -eq 0
'

run_shell required "16. Native machine fixture matrix" '
set -u
limit="${NCC_ANALYZE_MACHINE_LIMIT:-12}"
echo "limit=$limit per polarity"
failures=0

echo
echo "== expected OK native machine fixtures =="
count=0
while IFS= read -r f; do
  count=$((count + 1))
  printf "[OK:%03d] %s\n" "$count" "$f"
  if ! ncc_cmd machine check "$f" >/tmp/ncc_native_ok_stdout.txt 2>/tmp/ncc_native_ok_stderr.txt; then
    echo "UNEXPECTED FAIL: $f"
    cat /tmp/ncc_native_ok_stdout.txt
    cat /tmp/ncc_native_ok_stderr.txt
    failures=$((failures + 1))
  fi
done < <(find examples/machine/native -path "*/negative/*" -prune -o -type f -name "*.nm" -print | sort | head -n "$limit")

echo
echo "== expected FAIL native machine fixtures =="
count=0
while IFS= read -r f; do
  count=$((count + 1))
  printf "[FAIL:%03d] %s\n" "$count" "$f"
  if ncc_cmd machine check "$f" >/tmp/ncc_native_fail_stdout.txt 2>/tmp/ncc_native_fail_stderr.txt; then
    echo "UNEXPECTED PASS: $f"
    cat /tmp/ncc_native_fail_stdout.txt
    failures=$((failures + 1))
  fi
done < <(find examples/machine/native/negative -type f -name "*.nm" | sort | head -n "$limit")

rm -f /tmp/ncc_native_ok_stdout.txt /tmp/ncc_native_ok_stderr.txt /tmp/ncc_native_fail_stdout.txt /tmp/ncc_native_fail_stderr.txt
echo
echo "matrix_failures=$failures"
test "$failures" -eq 0
'

run_shell optional "17. Layer continuity static scan" '
set +e
limit="${NCC_ANALYZE_RG_LIMIT:-500}"
if command -v rg >/dev/null 2>&1 && rg --version >/dev/null 2>&1; then
  rg -n \
    "\[@machine\]|\[@\]|primitive\[@\]|Machine|machine|Terminator|ControlFlow|emit-ir|IR|Nir|Diagnostic|ptr<|memRegion|region|alias|link" \
    crates/ncc-ast crates/ncc-parser crates/ncc-sema crates/ncc-ir crates/ncc-cli examples/machine_at docs spec 2>/dev/null \
  | sed -n "1,${limit}p"
else
  grep -RInE "\[@machine\]|\[@\]|primitive\[@\]|Machine|machine|Terminator|ControlFlow|emit-ir|IR|Nir|Diagnostic|ptr<|memRegion|region|alias|link" \
    crates/ncc-ast crates/ncc-parser crates/ncc-sema crates/ncc-ir crates/ncc-cli examples/machine_at docs spec 2>/dev/null \
  | sed -n "1,${limit}p"
fi
'

run_shell optional "18. Parser AST sema IR touchpoints" '
set +e
limit="${NCC_ANALYZE_RG_LIMIT:-500}"
if command -v rg >/dev/null 2>&1 && rg --version >/dev/null 2>&1; then
  rg -n \
    "TokenKind|Lexer|Parser|Ast|AST|Program|sema|Semantic|Diagnostic|Lower|lower|IR|Nir|Backend|emit_backend|emit_ir" \
    crates/ncc-lexer crates/ncc-parser crates/ncc-ast crates/ncc-sema crates/ncc-ir crates/ncc-cli/src 2>/dev/null \
  | sed -n "1,${limit}p"
else
  grep -RInE "TokenKind|Lexer|Parser|Ast|AST|Program|sema|Semantic|Diagnostic|Lower|lower|IR|Nir|Backend|emit_backend|emit_ir" \
    crates/ncc-lexer crates/ncc-parser crates/ncc-ast crates/ncc-sema crates/ncc-ir crates/ncc-cli/src 2>/dev/null \
  | sed -n "1,${limit}p"
fi
'

run_shell optional "19. Debt and unsafe textual scan" '
set +e
limit="${NCC_ANALYZE_RG_LIMIT:-500}"
if command -v rg >/dev/null 2>&1 && rg --version >/dev/null 2>&1; then
  rg -n --hidden \
    -g "!target/**" -g "!.git/**" -g "!node_modules/**" -g "!.idea/**" -g "!reports/**" \
    "TODO|FIXME|HACK|todo!\(|unimplemented!\(|panic!\(|unwrap\(|expect\(|stub|placeholder|stringly|<expr>|unsafe|asm!" \
    . 2>/dev/null | sed -n "1,${limit}p"
else
  grep -RInE \
    --exclude-dir=target \
    --exclude-dir=.git \
    --exclude-dir=node_modules \
    --exclude-dir=.idea \
    --exclude-dir=reports \
    "TODO|FIXME|HACK|todo!\(|unimplemented!\(|panic!\(|unwrap\(|expect\(|stub|placeholder|stringly|<expr>|unsafe|asm!" . 2>/dev/null \
  | sed -n "1,${limit}p"
fi
'

run_shell optional "20. Example and fixture polarity inventory" '
set +e
find examples tests fixtures \
  -path "*/target" -prune -o \
  -type f \( -name "*.n" -o -name "*.nm" -o -name "*.nb" -o -name "*.ncp" -o -name "*.asm" \) -print 2>/dev/null \
| awk '"'"'
/_fail\./ || /\/negative\// { neg++ ; next }
/_ok\./ { ok++ ; next }
{ other++ }
END {
  printf "positive_named=%d\n", ok
  printf "negative_named=%d\n", neg
  printf "other_sources=%d\n", other
}
'"'"'
echo
echo "== sample positive =="
find examples tests fixtures -type f \( -name "*_ok.n" -o -name "*_ok.nm" -o -name "*_ok.nb" -o -name "*_ok.ncp" \) 2>/dev/null | sort | sed -n "1,80p"
echo
echo "== sample negative =="
find examples tests fixtures -type f \( -name "*_fail.n" -o -name "*_fail.nm" -o -name "*_fail.nb" -o -name "*_fail.ncp" \) 2>/dev/null | sort | sed -n "1,80p"
'

run_shell optional "21. Docs/status evidence around machine and continuity" '
set +e
find docs status spec handoff -type f \( -name "*.md" -o -name "*.json" \) 2>/dev/null \
| sort \
| grep -Ei "machine|primitive|runtime|compiler|basal|status|handoff" \
| sed -n "1,240p"
echo
if command -v rg >/dev/null 2>&1 && rg --version >/dev/null 2>&1; then
  rg -n "STATUS_machine|machine.*closed|deferred|parser|AST|sema|IR|runtime|primitive|basal" docs status spec handoff 2>/dev/null \
  | sed -n "1,240p"
fi
'

run_shell optional "22. Large artifacts and generated outputs" '
set +e
echo "== files over 20M, target excluded =="
find . \
  -path "./target" -prune -o \
  -path "./.git" -prune -o \
  -type f -size +20M -print 2>/dev/null | sort
echo
echo "== likely generated/report directories =="
find reports dist var target -maxdepth 3 -type f 2>/dev/null | sort | sed -n "1,240p"
'

run_shell optional "23. Existing project audit scripts" '
set +e
find tools/scripts tools/release tools/gates -maxdepth 2 -type f \( -name "*.sh" -o -name "*.py" \) 2>/dev/null | sort
'

section "24. Summary"
if [ "${#FAILURES[@]}" -eq 0 ]; then
  echo '```txt'
  echo "status=ok"
  echo "report=$OUT"
  echo '```'
  exit 0
fi

echo '```txt'
echo "status=failed"
echo "report=$OUT"
printf "failures=%d\n" "${#FAILURES[@]}"
for failure in "${FAILURES[@]}"; do
  echo "- $failure"
done
echo '```'
exit 1
