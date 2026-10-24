#!/usr/bin/env bash
set -u
set -o pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT" || exit 1

TS="$(date -u +%Y%m%dT%H%M%SZ)"
OUT="target/project-analysis-fast/$TS"
MD="$OUT/PROJECT_ANALYSIS_FAST.md"
ROWS="$OUT/status_rows.md"

mkdir -p "$OUT"/{summary,git,inventory,sizes,cargo,doctors,binaries,search}
: > "$ROWS"

OK=0
WARN=0
FAIL=0

record() {
  local code="$1"
  local label="$2"

  echo "| \`$code\` | \`$label.out\` |" >> "$ROWS"

  if [ "$code" = "0" ]; then
    OK=$((OK + 1))
  elif [ "$code" = "124" ]; then
    WARN=$((WARN + 1))
  else
    FAIL=$((FAIL + 1))
  fi
}

run() {
  local label="$1"
  local secs="$2"
  shift 2

  local file="$OUT/$label.out"
  mkdir -p "$(dirname "$file")"

  {
    echo "### COMMAND"
    printf '%q ' "$@"
    echo
    echo
    echo "### OUTPUT"
  } > "$file"

  timeout "$secs" "$@" >> "$file" 2>&1
  local code=$?

  record "$code" "$label"
}

run_sh() {
  local label="$1"
  local secs="$2"
  local script="$3"

  local file="$OUT/$label.out"
  mkdir -p "$(dirname "$file")"

  {
    echo "### SHELL"
    echo "$script"
    echo
    echo "### OUTPUT"
  } > "$file"

  timeout "$secs" bash -lc "$script" >> "$file" 2>&1
  local code=$?

  record "$code" "$label"
}

echo "[nC fast analysis] root: $ROOT"
echo "[nC fast analysis] out : $OUT"

# Base
run "summary/pwd" 10 pwd
run "summary/date_utc" 10 date -u
run "summary/uname" 10 uname -a

run_sh "summary/env_versions" 20 '
echo "git:"; git --version 2>/dev/null || true
echo "rustc:"; rustc --version 2>/dev/null || true
echo "cargo:"; cargo --version 2>/dev/null || true
echo "node:"; node --version 2>/dev/null || true
echo "npm:"; npm --version 2>/dev/null || true
echo "rg:"; rg --version 2>/dev/null | head -n 1 || true
echo "python:"; python3 --version 2>/dev/null || true
'

# Git
run "git/status_short" 15 git status --short
run "git/status_porcelain_v2" 15 git status --porcelain=v2
run "git/branch" 10 git branch --show-current
run "git/log_head" 15 git log --oneline -n 12
run "git/diff_stat" 30 git diff --stat
run "git/diff_name_status" 30 git diff --name-status
run "git/diff_check" 30 git diff --check

run_sh "git/counts" 15 '
echo "modified=$(git status --short | grep -E \"^ M|^M |^MM\" | wc -l)"
echo "untracked=$(git status --short | grep -E \"^\\?\\?\" | wc -l)"
echo "deleted=$(git status --short | grep -E \"^ D|^D \" | wc -l)"
'

# Inventario
run_sh "inventory/top_level_tree" 20 '
find . -maxdepth 2 -type d \
  ! -path "./.git*" \
  ! -path "./target*" \
  ! -path "./node_modules*" \
  ! -path "./site/node_modules*" \
  ! -path "./site/.next*" \
  | sort | sed "s#^\./##" | head -n 250
'

run_sh "inventory/file_counts_by_extension" 30 '
find . \
  -path ./.git -prune -o \
  -path ./target -prune -o \
  -path ./node_modules -prune -o \
  -path ./site/node_modules -prune -o \
  -path ./site/.next -prune -o \
  -type f -print \
| awk "
  {
    n=\$0
    sub(/^.*\\//, \"\", n)
    if (n !~ /\\./) ext=\"[no_ext]\"
    else { ext=n; sub(/^.*\\./, \".\", ext) }
    count[ext]++
  }
  END {
    for (e in count) print count[e], e
  }
" | sort -nr | head -n 80
'

run_sh "inventory/crates" 20 '
find crates -maxdepth 2 -name Cargo.toml -print 2>/dev/null | sort
'

run_sh "inventory/examples_docs_reports" 20 '
echo "examples files:"
find examples -type f 2>/dev/null | wc -l || true
echo "docs files:"
find docs -type f 2>/dev/null | wc -l || true
echo "reports files:"
find reports -type f 2>/dev/null | wc -l || true
'

# Tamaños
run_sh "sizes/top_dirs" 30 '
du -sh ./* 2>/dev/null | sort -hr | head -n 40
'

run_sh "sizes/largest_files" 30 '
find . \
  -path ./.git -prune -o \
  -path ./target -prune -o \
  -path ./node_modules -prune -o \
  -path ./site/node_modules -prune -o \
  -path ./site/.next -prune -o \
  -type f -printf "%s %p\n" 2>/dev/null \
| sort -nr | head -n 60 \
| awk "{printf \"%.2f MB  %s\\n\", \$1/1024/1024, \$2}"
'

# Cargo ligero
if command -v cargo >/dev/null 2>&1; then
  run "cargo/metadata_no_deps" 45 cargo metadata --no-deps --format-version 1
  run "cargo/tree_depth1" 45 cargo tree --workspace --depth 1

  run_sh "doctors/ncc_help" 60 'cargo run -q -p ncc-cli -- --help'
  run_sh "doctors/compiler_doctor" 60 'cargo run -q -p ncc-cli -- compiler doctor'
  run_sh "doctors/runtime_doctor" 60 'cargo run -q -p ncc-cli -- runtime doctor'
  run_sh "doctors/high_level_doctor_json" 60 'cargo run -q -p ncc-cli -- high-level doctor --json'
  run_sh "doctors/machine_native_doctor_json" 60 'cargo run -q -p ncc-cli -- machine asm-complete native-doctor --json'
  run_sh "doctors/basal_doctor_json" 60 'cargo run -q -p ncc-cli -- basal doctor --json'
  run_sh "doctors/imports_doctor_json" 60 'cargo run -q -p ncc-cli -- imports doctor --json'
  run_sh "doctors/mct_manifest_json" 60 'cargo run -q -p ncc-cli -- mct manifest --json'
  run_sh "doctors/backend_model_manifest_json" 60 'cargo run -q -p ncc-cli -- backend-model manifest --json'
  run_sh "doctors/ghor_doctor_json" 60 'cargo run -q -p ncc-cli -- ghor doctor --json'
else
  echo "cargo not found" > "$OUT/cargo/cargo_missing.out"
  record 127 "cargo/cargo_missing"
fi

# Binarios instalados
run_sh "binaries/which_all" 15 '
for b in ncc nclsp ghor nfmt nlint ncup nbuild; do
  printf "%-8s " "$b"
  if command -v "$b" >/dev/null 2>&1; then
    command -v "$b"
    "$b" --version 2>/dev/null || true
  else
    echo "MISSING"
  fi
done
'

# Búsquedas rápidas
if command -v rg >/dev/null 2>&1; then
  run_sh "search/todo_fixme_fake" 35 '
rg -n --hidden \
  --glob "!target/**" \
  --glob "!.git/**" \
  --glob "!node_modules/**" \
  --glob "!site/node_modules/**" \
  --glob "!site/.next/**" \
  "TODO|FIXME|unimplemented!|todo!|panic!|stub|fake" . \
  | head -n 300 || true
'

  run_sh "search/nc_keywords" 35 '
rg -n --hidden \
  --glob "!target/**" \
  --glob "!.git/**" \
  --glob "!node_modules/**" \
  --glob "!site/node_modules/**" \
  --glob "!site/.next/**" \
  "ptr<|memRegion|regionFromPtr|regionPtrAt|risk:|skkMain|ref main|primitive\\[@\\]|\\[@machine\\]|\\[@runtime\\]" . \
  | head -n 400 || true
'
else
  echo "rg missing" > "$OUT/search/rg_missing.out"
  record 127 "search/rg_missing"
fi

# Fallos
awk -F'|' '
  {
    code=$2
    gsub(/`/, "", code)
    gsub(/ /, "", code)
    if (code != "" && code != "0") print $0
  }
' "$ROWS" > "$OUT/summary/failing_commands.md"

# Reporte
{
  echo "# nC Fast Project Analysis"
  echo
  echo "- Status: \`completed\`"
  echo "- Timestamp UTC: \`$TS\`"
  echo "- Root: \`$ROOT\`"
  echo "- Output: \`$OUT\`"
  echo "- Mode: \`fast/read-only\`"
  echo
  echo "## Summary"
  echo
  echo "- OK commands: \`$OK\`"
  echo "- WARN/timeouts: \`$WARN\`"
  echo "- FAIL commands: \`$FAIL\`"
  echo
  echo "## Command Status"
  echo
  echo "| Exit | Artifact |"
  echo "| ---: | --- |"
  cat "$ROWS"
  echo
  echo "## Failing Commands"
  echo
  if [ -s "$OUT/summary/failing_commands.md" ]; then
    echo "| Exit | Artifact |"
    echo "| ---: | --- |"
    cat "$OUT/summary/failing_commands.md"
  else
    echo "No failing commands."
  fi
  echo
  echo "## Key Files"
  echo
  echo "- Git status: \`git/status_short.out\`"
  echo "- Git diff check: \`git/diff_check.out\`"
  echo "- Cargo metadata: \`cargo/metadata_no_deps.out\`"
  echo "- Doctors: \`doctors/\`"
  echo "- Binaries: \`binaries/which_all.out\`"
  echo "- Searches: \`search/\`"
  echo
  echo "Read-only: this script does not stage, commit, delete, clean or reset files."
} > "$MD"

cp "$MD" PROJECT_ANALYSIS_FAST_LATEST.md

echo
echo "[nC fast analysis] completed"
echo "[nC fast analysis] report: $MD"
echo "[nC fast analysis] latest: PROJECT_ANALYSIS_FAST_LATEST.md"
