#!/usr/bin/env bash
set -u
set -o pipefail

MODE="quick"
SHORT=20
MED=60
LONG=180
XLONG=420

for arg in "$@"; do
  case "$arg" in
    --quick) MODE="quick" ;;
    --full) MODE="full" ;;
    --deep) MODE="deep" ;;
    --short=*) SHORT="${arg#*=}" ;;
    --med=*) MED="${arg#*=}" ;;
    --long=*) LONG="${arg#*=}" ;;
    --xlong=*) XLONG="${arg#*=}" ;;
    -h|--help)
      echo "Usage: $0 [--quick|--full|--deep] [--short=N] [--med=N] [--long=N] [--xlong=N]"
      exit 0
      ;;
    *)
      echo "Unknown argument: $arg"
      exit 2
      ;;
  esac
done

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT" || exit 1

TS="$(date -u +%Y%m%dT%H%M%SZ)"
OUT="target/project-analysis-total/$TS"
MD="$OUT/PROJECT_ANALYSIS_TOTAL.md"
ROWS="$OUT/status_rows.md"

mkdir -p "$OUT"/{summary,env,git,inventory,sizes,cargo,doctors,binaries,site,search,examples,docs,reports}
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
  return 0
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
  return 0
}

echo "[nC total analysis] root: $ROOT"
echo "[nC total analysis] mode: $MODE"
echo "[nC total analysis] out : $OUT"

# ─────────────────────────────────────────────
# 1. Entorno
# ─────────────────────────────────────────────

run "env/pwd" "$SHORT" pwd
run "env/date_utc" "$SHORT" date -u
run "env/uname" "$SHORT" uname -a

run_sh "env/versions" "$SHORT" '
echo "== git =="; git --version 2>/dev/null || true
echo "== rustc =="; rustc --version 2>/dev/null || true
echo "== cargo =="; cargo --version 2>/dev/null || true
echo "== rustup =="; rustup --version 2>/dev/null || true
echo "== node =="; node --version 2>/dev/null || true
echo "== npm =="; npm --version 2>/dev/null || true
echo "== python3 =="; python3 --version 2>/dev/null || true
echo "== rg =="; rg --version 2>/dev/null | head -n 1 || true
echo "== jq =="; jq --version 2>/dev/null || true
echo "== zip =="; zip -v 2>/dev/null | head -n 1 || true
'

run_sh "env/path" "$SHORT" '
echo "$PATH" | tr ":" "\n"
'

# ─────────────────────────────────────────────
# 2. Git total
# ─────────────────────────────────────────────

run "git/status_short" "$SHORT" git status --short
run "git/status_porcelain_v2" "$SHORT" git status --porcelain=v2
run "git/status_ignored_short" "$SHORT" git status --ignored --short
run "git/branch" "$SHORT" git branch --show-current
run "git/remotes" "$SHORT" git remote -v
run "git/log_head" "$SHORT" git log --oneline -n 30
run "git/diff_stat" "$MED" git diff --stat
run "git/diff_name_status" "$MED" git diff --name-status
run "git/diff_check" "$MED" git diff --check
run "git/ls_files_modified" "$SHORT" git ls-files -m
run "git/ls_files_deleted" "$SHORT" git ls-files -d
run "git/ls_files_untracked" "$SHORT" git ls-files --others --exclude-standard

run_sh "git/worktree_summary" "$SHORT" '
echo "modified=$(git status --short | grep -E \"^ M|^M |^MM\" | wc -l)"
echo "untracked=$(git status --short | grep -E \"^\\?\\?\" | wc -l)"
echo "deleted=$(git status --short | grep -E \"^ D|^D \" | wc -l)"
echo "renamed=$(git status --short | grep -E \"^ R|^R \" | wc -l)"
echo "ignored=$(git status --ignored --short | grep -E \"^!!\" | wc -l)"
'

# ─────────────────────────────────────────────
# 3. Inventario
# ─────────────────────────────────────────────

run_sh "inventory/top_level_tree" "$MED" '
find . -maxdepth 3 -type d \
  ! -path "./.git*" \
  ! -path "./target*" \
  ! -path "./node_modules*" \
  ! -path "./site/node_modules*" \
  ! -path "./site/.next*" \
  | sort | sed "s#^\./##" | head -n 500
'

run_sh "inventory/file_counts_by_extension" "$MED" '
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
" | sort -nr | head -n 120
'

run_sh "inventory/crates" "$SHORT" '
find crates -maxdepth 3 -name Cargo.toml -print 2>/dev/null | sort
'

run_sh "inventory/source_counts" "$MED" '
echo "Rust files:"; find . -path ./target -prune -o -path ./.git -prune -o -name "*.rs" -type f -print | wc -l
echo "nC files:"; find . -path ./target -prune -o -path ./.git -prune -o \( -name "*.n" -o -name "*.nm" -o -name "*.ncp" -o -name "*.nb" -o -name "*.nbb" \) -type f -print | wc -l
echo "Markdown files:"; find . -path ./target -prune -o -path ./.git -prune -o -name "*.md" -type f -print | wc -l
echo "JSON files:"; find . -path ./target -prune -o -path ./.git -prune -o -name "*.json" -type f -print | wc -l
'

# ─────────────────────────────────────────────
# 4. Tamaños
# ─────────────────────────────────────────────

run_sh "sizes/top_dirs" "$MED" '
du -sh ./* 2>/dev/null | sort -hr | head -n 60
'

run_sh "sizes/largest_files" "$MED" '
find . \
  -path ./.git -prune -o \
  -path ./target -prune -o \
  -path ./node_modules -prune -o \
  -path ./site/node_modules -prune -o \
  -path ./site/.next -prune -o \
  -type f -printf "%s %p\n" 2>/dev/null \
| sort -nr | head -n 100 \
| awk "{printf \"%.2f MB  %s\\n\", \$1/1024/1024, \$2}"
'

# ─────────────────────────────────────────────
# 5. Docs / examples / reports
# ─────────────────────────────────────────────

run_sh "examples/examples_inventory" "$MED" '
if [ -d examples ]; then
  echo "examples total:"
  find examples -type f | wc -l
  echo
  echo "examples dirs:"
  find examples -maxdepth 4 -type d | sort | head -n 400
  echo
  echo "examples files:"
  find examples -type f | sort | head -n 500
else
  echo "no examples dir"
fi
'

run_sh "docs/docs_inventory" "$MED" '
if [ -d docs ]; then
  echo "docs total:"
  find docs -type f | wc -l
  echo
  find docs -type f | sort | head -n 700
else
  echo "no docs dir"
fi
'

run_sh "reports/reports_inventory" "$MED" '
if [ -d reports ]; then
  echo "reports total:"
  find reports -type f | wc -l
  echo
  find reports -maxdepth 5 -type f | sort | head -n 700
else
  echo "no reports dir"
fi
'

# ─────────────────────────────────────────────
# 6. Cargo / Rust
# ─────────────────────────────────────────────

if command -v cargo >/dev/null 2>&1; then
  run "cargo/metadata_no_deps" "$MED" cargo metadata --no-deps --format-version 1
  run "cargo/tree_depth1" "$MED" cargo tree --workspace --depth 1
  run "cargo/package_list" "$MED" cargo pkgid

  if [ "$MODE" = "full" ] || [ "$MODE" = "deep" ]; then
    run "cargo/check_workspace" "$LONG" cargo check --workspace
    run "cargo/test_workspace_no_run" "$LONG" cargo test --workspace --no-run
    run "cargo/clippy_workspace_no_deps" "$LONG" cargo clippy --workspace --no-deps -- -D warnings
  else
    echo "skipped in quick mode; run --full or --deep" > "$OUT/cargo/heavy_skipped_quick.out"
    record 0 "cargo/heavy_skipped_quick"
  fi

  if [ "$MODE" = "deep" ]; then
    run "cargo/test_workspace" "$XLONG" cargo test --workspace
  else
    echo "skipped unless --deep" > "$OUT/cargo/test_workspace_skipped_not_deep.out"
    record 0 "cargo/test_workspace_skipped_not_deep"
  fi
else
  echo "cargo not found" > "$OUT/cargo/cargo_missing.out"
  record 127 "cargo/cargo_missing"
fi

# ─────────────────────────────────────────────
# 7. Doctores nC
# ─────────────────────────────────────────────

if command -v cargo >/dev/null 2>&1; then
  run_sh "doctors/ncc_help" "$MED" 'cargo run -q -p ncc-cli -- --help'
  run_sh "doctors/ncc_version" "$MED" 'cargo run -q -p ncc-cli -- --version || cargo run -q -p ncc-cli -- version || true'
  run_sh "doctors/compiler_doctor" "$MED" 'cargo run -q -p ncc-cli -- compiler doctor'
  run_sh "doctors/runtime_doctor" "$MED" 'cargo run -q -p ncc-cli -- runtime doctor'
  run_sh "doctors/high_level_doctor_json" "$MED" 'cargo run -q -p ncc-cli -- high-level doctor --json'
  run_sh "doctors/machine_native_doctor_json" "$MED" 'cargo run -q -p ncc-cli -- machine asm-complete native-doctor --json'
  run_sh "doctors/basal_doctor_json" "$MED" 'cargo run -q -p ncc-cli -- basal doctor --json'
  run_sh "doctors/imports_doctor_json" "$MED" 'cargo run -q -p ncc-cli -- imports doctor --json'
  run_sh "doctors/mct_manifest_json" "$MED" 'cargo run -q -p ncc-cli -- mct manifest --json'
  run_sh "doctors/backend_model_manifest_json" "$MED" 'cargo run -q -p ncc-cli -- backend-model manifest --json'
  run_sh "doctors/ghor_doctor_json" "$MED" 'cargo run -q -p ncc-cli -- ghor doctor --json'
fi

# ─────────────────────────────────────────────
# 8. Binarios instalados
# ─────────────────────────────────────────────

run_sh "binaries/which_all" "$SHORT" '
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

# ─────────────────────────────────────────────
# 9. Site / web
# ─────────────────────────────────────────────

run_sh "site/site_inventory" "$MED" '
if [ -d site ]; then
  echo "site exists"
  find site -maxdepth 3 -type f \
    ! -path "site/node_modules/*" \
    ! -path "site/.next/*" \
    | sort | head -n 500
else
  echo "no site dir"
fi
'

run_sh "site/package_json" "$SHORT" '
if [ -f site/package.json ]; then
  cat site/package.json
else
  echo "no site/package.json"
fi
'

if [ "$MODE" = "deep" ]; then
  run_sh "site/npm_install_or_ci" "$XLONG" '
if [ -f site/package.json ]; then
  cd site
  if [ -f package-lock.json ]; then npm ci; else npm install; fi
else
  echo "no site package"
fi
'
  run_sh "site/npm_run_build" "$XLONG" '
if [ -f site/package.json ]; then
  cd site
  npm run build
else
  echo "no site package"
fi
'
else
  echo "skipped unless --deep" > "$OUT/site/site_build_skipped_not_deep.out"
  record 0 "site/site_build_skipped_not_deep"
fi

# ─────────────────────────────────────────────
# 10. Search gates
# ─────────────────────────────────────────────

if command -v rg >/dev/null 2>&1; then
  run_sh "search/todo_fixme_fake_panic" "$MED" '
rg -n --hidden \
  --glob "!target/**" \
  --glob "!.git/**" \
  --glob "!node_modules/**" \
  --glob "!site/node_modules/**" \
  --glob "!site/.next/**" \
  "TODO|FIXME|unimplemented!|todo!|panic!|stub|fake|placeholder" . \
  | head -n 800 || true
'

  run_sh "search/nc_core_keywords" "$MED" '
rg -n --hidden \
  --glob "!target/**" \
  --glob "!.git/**" \
  --glob "!node_modules/**" \
  --glob "!site/node_modules/**" \
  --glob "!site/.next/**" \
  "ptr<|memRegion|regionFromPtr|regionPtrAt|risk:|skkMain|ref main|primitive\\[@\\]|\\[@machine\\]|\\[@runtime\\]|\\[@kernel\\]|\\[@system\\]|\\[@inspect\\]" . \
  | head -n 1000 || true
'

  run_sh "search/error_diagnostics" "$MED" '
rg -n --hidden \
  --glob "!target/**" \
  --glob "!.git/**" \
  --glob "!node_modules/**" \
  --glob "!site/node_modules/**" \
  --glob "!site/.next/**" \
  "error\\[|warning\\[|E_[A-Z0-9_]+|W_[A-Z0-9_]+|diagnostic" crates docs examples reports \
  | head -n 1000 || true
'
else
  echo "rg missing" > "$OUT/search/rg_missing.out"
  record 127 "search/rg_missing"
fi

# ─────────────────────────────────────────────
# 11. Resumen de fallos
# ─────────────────────────────────────────────

awk -F'|' '
  {
    code=$2
    gsub(/`/, "", code)
    gsub(/ /, "", code)
    if (code != "" && code != "0") print $0
  }
' "$ROWS" > "$OUT/summary/failing_commands.md"

# ─────────────────────────────────────────────
# 12. Reporte principal
# ─────────────────────────────────────────────

{
  echo "# nC Total Project Analysis"
  echo
  echo "- Status: \`completed\`"
  echo "- Timestamp UTC: \`$TS\`"
  echo "- Root: \`$ROOT\`"
  echo "- Output: \`$OUT\`"
  echo "- Mode: \`$MODE\`"
  echo "- Read-only: \`true\`"
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
    echo
    echo "| Exit | Artifact |"
    echo "| ---: | --- |"
    cat "$OUT/summary/failing_commands.md"
  else
    echo
    echo "No failing commands."
  fi
  echo
  echo "## Key Files"
  echo
  echo "- Git status: \`git/status_short.out\`"
  echo "- Git diff check: \`git/diff_check.out\`"
  echo "- Inventory: \`inventory/\`"
  echo "- Sizes: \`sizes/\`"
  echo "- Cargo: \`cargo/\`"
  echo "- Doctors: \`doctors/\`"
  echo "- Binaries: \`binaries/which_all.out\`"
  echo "- Site: \`site/\`"
  echo "- Searches: \`search/\`"
  echo
  echo "This script does not stage, commit, delete, clean or reset files."
} > "$MD"

cp "$MD" PROJECT_ANALYSIS_TOTAL_LATEST.md

echo
echo "[nC total analysis] completed"
echo "[nC total analysis] report: $MD"
echo "[nC total analysis] latest: PROJECT_ANALYSIS_TOTAL_LATEST.md"
