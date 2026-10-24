#!/usr/bin/env bash
set -u

MODE="${1:-fast}"
TS="$(date +%Y%m%d_%H%M%S)"
OUT="reports/global_project_audit/nc_project_audit_${MODE}_${TS}.md"
mkdir -p "$(dirname "$OUT")"

run_block() {
  local title="$1"
  shift
  echo
  echo "## ${title}"
  echo
  echo '```txt'
  "$@" 2>&1 || true
  echo '```'
}

{
echo "# nC / ncc Project Audit — ${MODE}"
echo
echo "Fecha: $(date -Is)"
echo "Workspace: $(pwd)"
echo "Mode: ${MODE}"
echo

run_block "0. Environment" bash -lc '
uname -a
whoami
pwd
echo "JAVA_HOME=${JAVA_HOME:-}"
java -version || true
rustc --version || true
cargo --version || true
node -v || true
npm -v || true
'

run_block "1. Workspace sanity" bash -lc '
pwd
ls -lah Cargo.toml crates/ncc-cli/src/main.rs
find . -maxdepth 2 -type d | sort | sed -n "1,160p"
'

run_block "2. Git status / diff" bash -lc '
git status --short
echo
git diff --stat
echo
git diff --name-status
'

run_block "3. Cargo metadata" cargo metadata --no-deps --format-version 1

run_block "4. Cargo check" cargo check --workspace --all-targets

if [ "$MODE" = "full" ]; then
  run_block "5. Cargo fmt check" cargo fmt --all -- --check
  run_block "6. Cargo test workspace" cargo test --workspace --all-targets
  run_block "7. Cargo clippy workspace" cargo clippy --workspace --all-targets -- -D warnings
else
  echo
  echo "## 5. Cargo fmt/test/clippy"
  echo
  echo '```txt'
  echo "SKIPPED in fast mode. Run: ./tools/scripts/audit_nc_for_chatgpt.sh full"
  echo '```'
fi

run_block "8. ncc version / help" bash -lc '
cargo run -p ncc-cli -- --version || true
cargo run -p ncc-cli -- -V || true
cargo run -p ncc-cli -- --help || true
'

run_block "9. Compiler doctor / diagnostics" bash -lc '
cargo run -p ncc-cli -- compiler doctor || true
echo
cargo run -p ncc-cli -- compiler diagnostics --json || true
'

run_block "10. Runtime doctor / catalogs" bash -lc '
cargo run -p ncc-cli -- runtime doctor || true
echo
cargo run -p ncc-cli -- runtime families || true
echo
cargo run -p ncc-cli -- runtime capabilities --json || true
echo
cargo run -p ncc-cli -- runtime policies --json || true
echo
cargo run -p ncc-cli -- runtime errors --json || true
'

run_block "11. Runtime exports clave" bash -lc '
for f in \
  nC.runtime.input \
  nC.runtime.output \
  nC.runtime.render \
  nC.runtime.parse \
  nC.runtime.memory \
  nC.runtime.region \
  nC.runtime.heap \
  nC.runtime.queue \
  nC.runtime.list \
  nC.list \
  foreign.c \
  foreign.cpp \
  foreign.rust
do
  echo "===== $f ====="
  cargo run -p ncc-cli -- runtime exports "$f" || true
  echo
done
'

run_block "12. Primitive doctor / UART" bash -lc '
cargo run -p ncc-cli -- primitive doctor || true
echo
cargo run -p ncc-cli -- primitive target x86_64-unknown-none || true
echo
cargo run -p ncc-cli -- primitive target x86_64-protocol-lab || true
echo
if [ -f examples/primitive/hardware_identity_uart.ncp ]; then
  cargo run -p ncc-cli -- primitive check examples/primitive/hardware_identity_uart.ncp || true
  echo
  cargo run -p ncc-cli -- primitive witness examples/primitive/hardware_identity_uart.ncp --json || true
else
  echo "MISSING examples/primitive/hardware_identity_uart.ncp"
fi
'

run_block "13. Machine status intentionally deferred" bash -lc '
cargo run -p ncc-cli -- machine doctor || true
echo
echo "NOTE: [@machine] may be deliberately deferred until primitive[@], [@compiler] and [@runtime] are fully closed."
'

run_block "14. Main examples check/run" bash -lc '
FILES="
examples/helloWorld.n
examples/calculadoraBasica.n
examples/calculadoraElite.n
examples/randomArraySearchHidden.n
examples/entryRoot.nb
examples/runtime/entry.nb
examples/compiler/compiler_backend_target_report.n
examples/math/vectorCore.n
examples/primitive/hardware_identity_uart.ncp
"

for f in $FILES; do
  echo "===== $f ====="
  if [ -f "$f" ]; then
    echo "--- check ---"
    cargo run -p ncc-cli -- check "$f" || true

    case "$f" in
      *.ncp|*.nm|*.nbb)
        echo "--- run ---"
        echo "SKIP run for low-level/source-specific file"
        ;;
      *)
        echo "--- run ---"
        timeout 20s cargo run -p ncc-cli -- run "$f" || true
        ;;
    esac
  else
    echo "MISSING"
  fi
  echo
done
'

run_block "15. Arrays / lists / bounds / type{} / FFI surface" bash -lc '
rg -n "array<|int\\[|list<|nC\\.runtime\\.list|nC\\.list|bounds|type:|constructure|destructure|copy\\.|clone\\.|dinArray|matrix|tensor|xy<|xyz<|foreign\\.c|foreign\\.cpp|foreign\\.rust|panicBoundary|exceptionBoundary|ffi" crates examples tests docs tools 2>/dev/null | sed -n "1,360p" || true
'

run_block "16. Plugin VS Code" bash -lc '
if [ -d tools/vscode/nc-language ]; then
  cd tools/vscode/nc-language
  echo "PWD=$(pwd)"
  ls -lah package.json README.md MARKETPLACE.md LICENSE* 2>/dev/null || true
  npm run test || true
  if [ "$MODE" = "full" ]; then
    npm run package:vsix || true
  else
    echo "SKIP package:vsix in fast mode"
  fi
  find dist -maxdepth 2 -type f -print 2>/dev/null || true
else
  echo "MISSING tools/vscode/nc-language"
fi
'

run_block "17. Plugin JetBrains" bash -lc '
if [ -d tools/jetbrains/nc-language-plugin ]; then
  cd tools/jetbrains/nc-language-plugin
  echo "PWD=$(pwd)"
  ls -lah build.gradle.kts settings.gradle.kts gradle.properties README.md MARKETPLACE.md LICENSE* 2>/dev/null || true
  ./gradlew --version || true
  if [ "$MODE" = "full" ]; then
    ./gradlew clean build buildPlugin --no-daemon || true
  else
    ./gradlew buildPlugin --no-daemon || true
  fi
  find build/distributions -type f -name "*.zip" -print 2>/dev/null || true
  rg -n "org.jetbrains.intellij|org.jetbrains.intellij.platform|sinceBuild|untilBuild|jvmTarget|JavaLanguageVersion|VERSION_17|VERSION_21|VERSION_25|261|252|242" . 2>/dev/null || true
else
  echo "MISSING tools/jetbrains/nc-language-plugin"
fi
'

run_block "18. Ghidra plugin" bash -lc '
if [ -d tools/ghidra ]; then
  find tools/ghidra -maxdepth 5 -type f | sort
  echo
  rg -n "nc-logo|nc_colors|MIT OR Apache-2.0|Module.manifest|extension.properties" tools/ghidra 2>/dev/null || true
else
  echo "MISSING tools/ghidra"
fi
'

run_block "19. Theme / assets / logo" bash -lc '
find tools/theme tools/assets tools/vscode/nc-language/assets tools/vscode/nc-language/themes tools/vscode/nc-language/icons docs/assets -maxdepth 5 -type f 2>/dev/null | sort
echo
rg -n "#0A134A|#A4568D|#49399C|#190C1F|#034A83|#43083C|#891253|#210F4A|#200E33|#8C3A76|#FFFFFF|#D3ADA7|#9BD5F0|#EA6062|#F94144|nc-sovereign-dark|nc-logo|iconThemes" tools docs 2>/dev/null | sed -n "1,260p" || true
'

run_block "20. License audit" bash -lc '
rg -n "\"license\"|MIT OR Apache-2.0|Apache-2.0|LICENSE|SPDX-License-Identifier" Cargo.toml crates tools docs package.json 2>/dev/null | sed -n "1,260p" || true
find . -maxdepth 5 -type f \( -name "LICENSE" -o -name "LICENSE-MIT" -o -name "LICENSE-APACHE" \) -print | sort
'

run_block "21. Docs/status/reports index" bash -lc '
find docs/status docs/spec/release reports -maxdepth 3 -type f 2>/dev/null | sort | sed -n "1,320p"
'

run_block "22. Technical debt scan" bash -lc '
rg -n "TODO|FIXME|HACK|unimplemented!|todo!|panic!|stub|placeholder|<expr>|String\\(\"<expr>\"\\)" crates examples tests docs tools 2>/dev/null | sed -n "1,320p" || true
'

run_block "23. Artifacts / large files / generated dirs" bash -lc '
echo "== Files >30M =="
find . -type f -size +30M -print 2>/dev/null | sort
echo
echo "== Build artifacts packages =="
find . -type f \( -name "*.vsix" -o -name "*.zip" -o -name "*.jar" -o -name "*.wasm" -o -name "*.elf" -o -name "*.o" -o -name "*.bin" \) -print 2>/dev/null | sort
echo
echo "== Generated dirs =="
find . -type d \( -name node_modules -o -name target -o -name build -o -name .gradle -o -name .intellijPlatform -o -name .vscode-test -o -name __pycache__ -o -name .pytest_cache \) -print 2>/dev/null | sort
'

run_block "24. Possible secrets scan" bash -lc '
rg -n "TOKEN|SECRET|PASSWORD|API_KEY|PRIVATE_KEY|BEGIN RSA|BEGIN OPENSSH|AZURE_DEVOPS_EXT_PAT|VSCE_PAT|OVSX_PAT|\\.env" . 2>/dev/null | sed -n "1,180p" || true
'

run_block "25. Final git status" bash -lc '
git status --short
echo
git diff --stat
'

} | tee "$OUT"

echo
echo "Reporte generado: $OUT"
