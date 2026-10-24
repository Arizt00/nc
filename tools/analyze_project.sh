#!/usr/bin/env bash
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
OUT="${ROOT}/target/project-analysis/${STAMP}"
LOG="${OUT}/analysis.log"
REPORT_MD="${OUT}/PROJECT_ANALYSIS.md"
LATEST_MD="${ROOT}/target/project-analysis/PROJECT_ANALYSIS_LATEST.md"
QUICK=0

for arg in "$@"; do
  case "${arg}" in
    --quick)
      QUICK=1
      ;;
    -h|--help)
      cat <<'EOF'
Usage: tools/analyze_project.sh [--quick]

Creates a read-only project analysis bundle under:
  target/project-analysis/<timestamp>/

Outputs a live Markdown report:
  target/project-analysis/<timestamp>/PROJECT_ANALYSIS.md

Options:
  --quick   Skip heavyweight cargo/npm validation and collect inventory,
            git state, doctors, search gates and summaries faster.
EOF
      exit 0
      ;;
    *)
      echo "unknown option: ${arg}" >&2
      exit 2
      ;;
  esac
done

mkdir -p \
  "${OUT}/git" \
  "${OUT}/cargo" \
  "${OUT}/doctors" \
  "${OUT}/inventory" \
  "${OUT}/search" \
  "${OUT}/sizes" \
  "${OUT}/binaries" \
  "${OUT}/site" \
  "${OUT}/summary"

write_live_report() {
  local state="${1:-running}"
  {
    echo "# nC Project Analysis"
    echo
    echo "- Status: \`${state}\`"
    echo "- Timestamp UTC: \`${STAMP}\`"
    echo "- Root: \`${ROOT}\`"
    echo "- Output: \`${OUT}\`"
    echo "- Mode: \`$([ "${QUICK}" -eq 1 ] && echo quick || echo full)\`"
    echo
    echo "## Live Command Status"
    echo
    echo "| Exit | Artifact |"
    echo "| ---: | --- |"
    if [ -f "${OUT}/summary/command_status.tsv" ]; then
      awk -F "\t" 'NR > 1 { printf "| `%s` | `%s` |\n", $1, $2 }' "${OUT}/summary/command_status.tsv"
    fi
    echo
    echo "## Key Artifacts"
    echo
    echo "- Git status: \`git/status_short.txt\`"
    echo "- Domain summary: \`summary/worktree_domain_summary.txt\`"
    echo "- Failing commands: \`summary/failing_commands.txt\`"
    echo "- Cargo check: \`cargo/check_workspace.out\`"
    echo "- Cargo no-run tests: \`cargo/test_workspace_no_run.out\`"
    echo "- Clippy: \`cargo/clippy_workspace.out\`"
    echo "- Doctors: \`doctors/\`"
    echo "- Search gates: \`search/\`"
    echo "- SHA256: \`summary/sha256sums.txt\`"
    echo
    echo "This script is read-only with respect to source code. It does not stage,"
    echo "commit, delete, clean or reset files."
  } > "${REPORT_MD}"
  cp "${REPORT_MD}" "${LATEST_MD}" 2>/dev/null || true
}

finish_interrupted() {
  echo "analysis interrupted: ${OUT}" | tee -a "${LOG}"
  write_live_report "interrupted"
  exit 130
}

trap finish_interrupted INT TERM

run_capture() {
  local name="$1"
  shift
  local file="${OUT}/${name}"
  mkdir -p "$(dirname "${file}")"
  echo "[RUN] ${name}"
  {
    echo "\$ $*"
    echo
    "$@"
  } >"${file}" 2>&1
  local status=$?
  printf "%s\t%s\n" "${status}" "${name}" >> "${OUT}/summary/command_status.tsv"
  if [ "${status}" -eq 0 ]; then
    echo "[OK]  ${name}"
  else
    echo "[ERR] ${name} exit=${status} (see ${file})"
  fi
  write_live_report "running"
  return 0
}

run_shell_capture() {
  local name="$1"
  shift
  local file="${OUT}/${name}"
  mkdir -p "$(dirname "${file}")"
  echo "[RUN] ${name}"
  {
    echo "\$ $*"
    echo
    bash -lc "$*"
  } >"${file}" 2>&1
  local status=$?
  printf "%s\t%s\n" "${status}" "${name}" >> "${OUT}/summary/command_status.tsv"
  if [ "${status}" -eq 0 ]; then
    echo "[OK]  ${name}"
  else
    echo "[ERR] ${name} exit=${status} (see ${file})"
  fi
  write_live_report "running"
  return 0
}

has_cmd() {
  command -v "$1" >/dev/null 2>&1
}

echo "nC project analysis started: ${STAMP}" | tee "${LOG}"
echo "root: ${ROOT}" | tee -a "${LOG}"
echo "output: ${OUT}" | tee -a "${LOG}"
echo -e "status\tartifact" > "${OUT}/summary/command_status.tsv"
write_live_report "running"

cd "${ROOT}" || exit 1

# ---------------------------------------------------------------------------
# Environment
# ---------------------------------------------------------------------------
run_capture "summary/pwd.txt" pwd
run_capture "summary/uname.txt" uname -a
run_capture "summary/date_utc.txt" date -u
run_capture "summary/env_key_versions.txt" bash -lc '
set +e
echo "rustc: $(rustc --version 2>/dev/null)"
echo "cargo: $(cargo --version 2>/dev/null)"
echo "git:   $(git --version 2>/dev/null)"
echo "node:  $(node --version 2>/dev/null)"
echo "npm:   $(npm --version 2>/dev/null)"
echo "clang: $(clang --version 2>/dev/null | head -1)"
echo "llvm-as: $(llvm-as --version 2>/dev/null | head -1)"
echo "llc: $(llc --version 2>/dev/null | head -1)"
echo "ld.lld: $(ld.lld --version 2>/dev/null | head -1)"
'

# ---------------------------------------------------------------------------
# Git state
# ---------------------------------------------------------------------------
run_capture "git/status_short.txt" git status --short
run_capture "git/status_ignored_short.txt" git status --ignored --short
run_capture "git/status_porcelain_v2.txt" git status --porcelain=v2 --branch
run_capture "git/diff_stat.txt" git diff --stat
run_capture "git/diff_name_status.txt" git diff --name-status
run_capture "git/diff_check.txt" git diff --check
run_capture "git/modified.txt" git ls-files --modified
run_capture "git/deleted.txt" git ls-files --deleted
run_capture "git/untracked.txt" git ls-files --others --exclude-standard
run_capture "git/ignored.txt" git ls-files --others --ignored --exclude-standard
run_capture "git/tracked_files.txt" git ls-files

# ---------------------------------------------------------------------------
# Inventory
# ---------------------------------------------------------------------------
run_shell_capture "inventory/file_counts_by_extension.txt" '
git ls-files | awk "
  function ext(path) {
    n=split(path,a,\"/\"); f=a[n];
    if (f !~ /\\./) return \"[no_ext]\";
    sub(/^.*\\./,\"\",f); return \".\" f;
  }
  { c[ext($0)]++ }
  END { for (e in c) print c[e], e }
" | sort -nr
'

run_shell_capture "inventory/top_level_tree.txt" '
find . -maxdepth 2 -mindepth 1 \
  -not -path "./target*" \
  -not -path "./.git*" \
  -printf "%y %p\n" | sort
'

run_shell_capture "inventory/crates.txt" '
find crates -maxdepth 3 -name Cargo.toml -print 2>/dev/null | sort
'

run_shell_capture "inventory/examples_by_mode.txt" '
find examples -type f 2>/dev/null | awk "
  /\\.n$/ { n++ }
  /\\.nb$/ { nb++ }
  /\\.nm$/ { nm++ }
  /\\.ncp$/ { ncp++ }
  END {
    print \".n\", n+0;
    print \".nb\", nb+0;
    print \".nm\", nm+0;
    print \".ncp\", ncp+0;
  }
"
'

run_shell_capture "inventory/docs_reports_counts.txt" '
echo "docs files: $(find docs -type f 2>/dev/null | wc -l)"
echo "reports files: $(find reports -type f 2>/dev/null | wc -l)"
echo "examples files: $(find examples -type f 2>/dev/null | wc -l)"
echo "crates files: $(find crates -type f 2>/dev/null | wc -l)"
echo "tools files: $(find tools -type f 2>/dev/null | wc -l)"
'

# ---------------------------------------------------------------------------
# Size and large-file scan
# ---------------------------------------------------------------------------
run_shell_capture "sizes/top_dirs.txt" '
du -h --max-depth=2 . 2>/dev/null | sort -hr | head -120
'

run_shell_capture "sizes/largest_files.txt" '
find . -type f \
  -not -path "./.git/*" \
  -not -path "./target/debug/*" \
  -not -path "./target/release/*" \
  -printf "%s %p\n" 2>/dev/null | sort -nr | head -120
'

# ---------------------------------------------------------------------------
# Cargo validation and metadata
# ---------------------------------------------------------------------------
run_capture "cargo/metadata.json" cargo metadata --format-version 1 --no-deps
run_capture "cargo/tree.txt" cargo tree
if [ "${QUICK}" -eq 0 ]; then
  run_capture "cargo/fmt_check.out" cargo fmt --all -- --check
  run_capture "cargo/check_workspace.out" cargo check --workspace --all-targets
  run_capture "cargo/test_workspace_no_run.out" cargo test --workspace --all-targets --no-run
  run_capture "cargo/clippy_workspace.out" cargo clippy --workspace --all-targets -- -D warnings
else
  printf "%s\t%s\n" "0" "cargo/heavy_validation_skipped_quick_mode" >> "${OUT}/summary/command_status.tsv"
  echo "Skipped cargo fmt/check/test --no-run/clippy because --quick was requested." \
    > "${OUT}/cargo/heavy_validation_skipped_quick_mode.txt"
  write_live_report "running"
fi

# ---------------------------------------------------------------------------
# Project doctors and command surfaces
# ---------------------------------------------------------------------------
for cmd in \
  "cargo run -q -p ncc-cli -- --help" \
  "cargo run -q -p ncc-cli -- compiler doctor" \
  "cargo run -q -p ncc-cli -- runtime doctor" \
  "cargo run -q -p ncc-cli -- high-level doctor --json" \
  "cargo run -q -p ncc-cli -- machine asm-complete native-doctor --json" \
  "cargo run -q -p ncc-cli -- basal doctor --json" \
  "cargo run -q -p ncc-cli -- mct manifest --json" \
  "cargo run -q -p ncc-cli -- imports doctor --json" \
  "cargo run -q -p ncc-cli -- backend-model manifest --json" \
  "cargo run -q -p ncc-cli -- ghor doctor --json"
do
  safe="$(echo "${cmd}" | tr ' /:' '___' | tr -cd 'A-Za-z0-9_.-')"
  run_shell_capture "doctors/${safe}.out" "${cmd}"
done

# ---------------------------------------------------------------------------
# Installed/local binary smoke
# ---------------------------------------------------------------------------
for bin in ncc nclsp ghor nfmt nlint ncup nbuild; do
  if has_cmd "${bin}"; then
    run_capture "binaries/${bin}_which.txt" which "${bin}"
    run_capture "binaries/${bin}_help.txt" "${bin}" --help
  else
    echo "127 binaries/${bin}_missing" >> "${OUT}/summary/command_status.tsv"
    echo "${bin}: missing" > "${OUT}/binaries/${bin}_missing.txt"
  fi
done

# ---------------------------------------------------------------------------
# Site/plugin checks, if present
# ---------------------------------------------------------------------------
if [ -f "package.json" ]; then
  run_capture "site/npm_root_package_json.txt" cat package.json
  if has_cmd npm; then
    run_capture "site/npm_ls_root.out" npm ls --depth=0
  fi
fi

if [ -d "site" ]; then
  run_shell_capture "site/site_inventory.txt" 'find site -maxdepth 3 -type f | sort | head -500'
  if [ -f "site/package.json" ]; then
    run_capture "site/package_json.txt" cat site/package.json
    if has_cmd npm; then
      run_shell_capture "site/npm_ls_site.out" 'cd site && npm ls --depth=0'
      if [ "${QUICK}" -eq 0 ]; then
        run_shell_capture "site/npm_run_build_site.out" 'cd site && npm run build'
      else
        printf "%s\t%s\n" "0" "site/npm_run_build_site_skipped_quick_mode" >> "${OUT}/summary/command_status.tsv"
        echo "Skipped site npm build because --quick was requested." \
          > "${OUT}/site/npm_run_build_site_skipped_quick_mode.txt"
        write_live_report "running"
      fi
    fi
  fi
fi

if [ -d "editors" ] || [ -d "plugins" ] || [ -d ".vscode" ]; then
  run_shell_capture "site/plugin_inventory.txt" '
for d in editors plugins .vscode; do
  [ -d "$d" ] && find "$d" -maxdepth 4 -type f | sort
done
'
fi

# ---------------------------------------------------------------------------
# Search gates
# ---------------------------------------------------------------------------
SEARCH_SCOPE="crates examples docs reports tools site .github"
SEARCH_PATTERNS=(
  "fake evidence"
  "fake machine evidence"
  "fake seal"
  "fake nbb"
  "json disguised"
  "placeholder"
  "TODO"
  "FIXME"
  "stub"
  "unimplemented!"
  "todo!"
  "panic!"
  "unsupported.*but ok"
  "blocked_by_machine"
  "machine.*blocked"
)

if has_cmd rg; then
  for pat in "${SEARCH_PATTERNS[@]}"; do
    safe="$(echo "${pat}" | tr -cd 'A-Za-z0-9_-' | head -60)"
    run_shell_capture "search/${safe:-pattern}.txt" "rg -n --hidden -S \"${pat}\" ${SEARCH_SCOPE} 2>/dev/null || true"
  done
else
  cat > "${OUT}/search/README.txt" <<'MSG'
rg was not available. Falling back to grep -R for simple literal checks where possible.
Regex-specific patterns may be less precise.
MSG
  for pat in "${SEARCH_PATTERNS[@]}"; do
    safe="$(echo "${pat}" | tr -cd 'A-Za-z0-9_-' | head -60)"
    run_shell_capture "search/${safe:-pattern}.txt" "grep -RIn \"${pat}\" ${SEARCH_SCOPE} 2>/dev/null || true"
  done
fi

# ---------------------------------------------------------------------------
# Classify paths into broad domains.
# ---------------------------------------------------------------------------
run_shell_capture "summary/worktree_domain_summary.txt" '
git status --short | awk "
  function cat(path) {
    if (path ~ /^Cargo\\.(toml|lock)$/) return \"root_cargo\";
    if (path ~ /^crates\\//) return \"core_crates\";
    if (path ~ /^examples\\/machine\\// || path ~ /^docs\\/.*machine/ || path ~ /^reports\\/machine/) return \"machine\";
    if (path ~ /^examples\\/basal\\// || path ~ /^docs\\/.*basal/ || path ~ /^reports\\/basal/) return \"basal\";
    if (path ~ /^examples\\/primitive\\// || path ~ /^docs\\/.*primitive/) return \"primitive\";
    if (path ~ /^examples\\/runtime\\// || path ~ /^docs\\/.*runtime/) return \"runtime\";
    if (path ~ /^examples\\/high_level\\// || path ~ /^docs\\/.*high/) return \"high_level\";
    if (path ~ /^site\\//) return \"site_web\";
    if (path ~ /^docs\\/status\\//) return \"docs_status\";
    if (path ~ /^docs\\//) return \"docs\";
    if (path ~ /^reports\\//) return \"reports\";
    if (path ~ /^target\\/release-evidence\\//) return \"release_evidence\";
    if (path ~ /^tools\\//) return \"tools\";
    if (path ~ /^examples\\//) return \"examples\";
    if (path ~ /^\\.github\\//) return \"ci\";
    return \"unclassified\";
  }
  {
    code=substr($0,1,2);
    path=substr($0,4);
    gsub(/^\"|\"$/, \"\", path);
    c=cat(path);
    count[c]++;
  }
  END { for (c in count) print count[c], c }
" | sort -nr
'

# ---------------------------------------------------------------------------
# Final summary
# ---------------------------------------------------------------------------
run_shell_capture "summary/failing_commands.txt" '
awk -F "\t" "NR > 1 && \$1 != 0 { print }" target/project-analysis/'"${STAMP}"'/summary/command_status.tsv
'

run_shell_capture "summary/sha256sums.txt" '
find target/project-analysis/'"${STAMP}"' -type f -not -name sha256sums.txt -print0 |
  sort -z |
  xargs -0 sha256sum
'

write_live_report "complete"
cp "${REPORT_MD}" "${OUT}/README.md"

echo "analysis complete: ${OUT}" | tee -a "${LOG}"
echo "read: ${REPORT_MD}" | tee -a "${LOG}"
echo "latest: ${LATEST_MD}" | tee -a "${LOG}"
