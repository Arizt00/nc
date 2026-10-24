#!/usr/bin/env bash
set +e

TS="$(date +%Y%m%d_%H%M%S)"
OUT="reports/audit/nc_language_full_audit_${TS}.md"
ZIP="reports/audit/nc_language_full_audit_${TS}.zip"

mkdir -p reports/audit

exec > >(tee "$OUT") 2>&1

section() {
  echo
  echo "## $1"
  echo
}

run_cmd() {
  local title="$1"
  shift
  section "$title"
  echo '```bash'
  echo "$*"
  echo '```'
  echo
  echo '```txt'
  "$@"
  local code=$?
  echo
  echo "[exit-code: $code]"
  echo '```'
}

scan_rg() {
  local title="$1"
  local pattern="$2"
  section "$title"
  echo '```txt'
  if command -v rg >/dev/null 2>&1; then
    rg -n --hidden \
      -g '!target' \
      -g '!reports' \
      -g '!var' \
      -g '!tmp' \
      -g '!cache' \
      -g '!.git' \
      -g '!node_modules' \
      -g '!*.zip' \
      -g '!*.tar' \
      -g '!*.tar.gz' \
      "$pattern" . || true
  else
    find . \
      -path './target' -prune -o \
      -path './reports' -prune -o \
      -path './var' -prune -o \
      -path './tmp' -prune -o \
      -path './cache' -prune -o \
      -path './.git' -prune -o \
      -type f -print0 \
    | xargs -0 grep -RInE "$pattern" 2>/dev/null || true
  fi
  echo '```'
}

echo "# nC / ncc — Auditoría completa del lenguaje"
echo
echo "- Fecha: $(date -Is)"
echo "- Workspace: $(pwd)"
echo "- Usuario: $(whoami)"
echo "- Sistema: $(uname -a)"
echo

section "0. Herramientas disponibles"
for t in git rustc cargo rustfmt clippy-driver rg tree find awk sed grep wc du zip; do
  if command -v "$t" >/dev/null 2>&1; then
    echo "- ✅ $t -> $(command -v "$t")"
  else
    echo "- ❌ $t no encontrado"
  fi
done

section "1. Tamaño total real del proyecto"
echo '```txt'
du -sh .
echo '```'

section "2. Peso por carpetas — nivel 1"
echo '```txt'
du -h --max-depth=1 . 2>/dev/null | sort -h
echo '```'

section "3. Peso por carpetas — nivel 2, top 120"
echo '```txt'
du -h --max-depth=2 . 2>/dev/null | sort -h | tail -n 120
echo '```'

section "4. Archivos más pesados — top 120"
echo '```txt'
find . -type f -printf "%s\t%p\n" 2>/dev/null \
  | sort -nr \
  | head -n 120 \
  | awk '{printf "%.2f MB\t%s\n", $1/1024/1024, $2}'
echo '```'

section "5. Tamaño source-only aproximado sin target/reports/var/cache"
echo '```txt'
du -ch \
  --exclude='./target' \
  --exclude='./reports' \
  --exclude='./var' \
  --exclude='./tmp' \
  --exclude='./cache' \
  --exclude='./.git' \
  --exclude='./node_modules' \
  . 2>/dev/null | tail -n 1
echo '```'

run_cmd "6. Git status" git status --short --branch
run_cmd "7. Últimos commits" git log --oneline --decorate --graph -n 25
run_cmd "8. Cargo metadata" cargo metadata --no-deps --format-version 1
run_cmd "9. Cargo tree workspace" cargo tree --workspace

section "10. Estructura de directorios relevante"
echo '```txt'
if command -v tree >/dev/null 2>&1; then
  tree -a -L 5 \
    -I "target|reports|var|tmp|cache|.git|node_modules|*.zip|*.tar|*.tar.gz" .
else
  find . \
    -path './target' -prune -o \
    -path './reports' -prune -o \
    -path './var' -prune -o \
    -path './tmp' -prune -o \
    -path './cache' -prune -o \
    -path './.git' -prune -o \
    -type d -maxdepth 5 -print | sort
fi
echo '```'

section "11. Conteo por extensión, sin generado pesado"
echo '```txt'
find . \
  -path './target' -prune -o \
  -path './reports' -prune -o \
  -path './var' -prune -o \
  -path './tmp' -prune -o \
  -path './cache' -prune -o \
  -path './.git' -prune -o \
  -type f -print \
| awk '
  {
    n=$0
    if (n !~ /\./) ext="[sin_ext]";
    else { sub(/^.*\./,"",n); ext=n; }
    count[ext]++
  }
  END {
    for (e in count) print count[e], e
  }
' | sort -nr
echo '```'

section "12. Líneas por archivos relevantes — top 100"
echo '```txt'
find . \
  -path './target' -prune -o \
  -path './reports' -prune -o \
  -path './var' -prune -o \
  -path './tmp' -prune -o \
  -path './cache' -prune -o \
  -path './.git' -prune -o \
  -type f \( \
    -name "*.rs" -o \
    -name "*.n" -o \
    -name "*.nb" -o \
    -name "*.nm" -o \
    -name "*.ncp" -o \
    -name "*.md" -o \
    -name "*.toml" -o \
    -name "*.json" -o \
    -name "*.sh" \
  \) -print0 \
| xargs -0 wc -l 2>/dev/null \
| sort -nr \
| head -n 100
echo '```'

run_cmd "13. cargo fmt check" cargo fmt --all -- --check
run_cmd "14. cargo check workspace" cargo check --workspace --all-targets
run_cmd "15. cargo test workspace" cargo test --workspace --all-targets
run_cmd "16. cargo clippy workspace" cargo clippy --workspace --all-targets -- -D warnings

section "17. Binarios generados o disponibles"
echo '```txt'
find target/debug -maxdepth 3 -type f -perm -111 2>/dev/null | sort || true
echo '```'

section "18. ncc / ncup si existen en PATH o target"
echo '```txt'
if command -v ncc >/dev/null 2>&1; then
  echo "which ncc: $(command -v ncc)"
  ncc --version 2>&1 || true
  ncc --help 2>&1 | head -n 80 || true
else
  echo "ncc no está en PATH"
fi

echo

if [ -x "./target/debug/ncc" ]; then
  echo "./target/debug/ncc existe"
  ./target/debug/ncc --version 2>&1 || true
  ./target/debug/ncc --help 2>&1 | head -n 80 || true
fi

echo

if command -v ncup >/dev/null 2>&1; then
  echo "which ncup: $(command -v ncup)"
  ncup doctor 2>&1 || true
else
  echo "ncup no está en PATH"
fi
echo '```'

scan_rg "19. TODO/FIXME/panic/unwrap/expect/stubs" "TODO|FIXME|HACK|panic!|unwrap\\(|expect\\(|todo!\\(|unimplemented!\\(|stub|placeholder"
scan_rg "20. Núcleo compilador: lexer/parser/AST/sema/nIR/backend/diagnostics/span" "lexer|Lexer|TokenKind|parser|Parser|AST|Ast|sema|semantic|nIR|nir|IR|backend|lowering|Diagnostic|diagnostic|Span|span"
scan_rg "21. Modos nC oficiales" "\\[@\\]|\\[@runtime\\]|\\[@system\\]|\\[@kernel\\]|\\[@machine\\]|\\[@network\\]|\\[@security\\]|\\[@inspect\\]|\\[@tooling\\]|\\[@compiler\\]|\\[@ai\\]|primitive\\[@\\]"
scan_rg "22. Runtime imports y familias" "use nC\\.runtime|nC\\.runtime\\.|runtime\\.input|runtime\\.output|runtime\\.render|runtime\\.parse|runtime\\.validate|runtime\\.result|runtime_families|runtime_api"
scan_rg "23. Familia print completa" "print\\.sensory|print\\.mmio|print\\.port|print\\.dump|print\\.addr|print\\.phys|print\\.peek|print\\.stable|print\\.inspect|print\\.mem|print\\.site|print\\.type|print\\.raw|print\\.trc|print\\.dbg|print\\.wm|print\\.err|print\\.ok|print\\.sta"
scan_rg "24. Root/basal [@]: binary/bind/raw/final/skk/alias/root memory" "binary:|bind:|raw:|final:|fa:|skk|memory\\.root|root\\.memory|rootPtr|aliasPtr|memRegion|ptr<|Root"
scan_rg "25. Machine/hardware: MMIO/port/volatile/fence/register/ABI/linker/QEMU" "MMIO|mmio|portRead|portWrite|print\\.port|volatile|fence|atomic|register|x86_64|riscv|aarch64|ABI|calling convention|linker|qemu|freestanding|unknown-none|no_std|asm!"
scan_rg "26. Primitive: anchor/slot/window/signal/bus/timing/seal/sensory" "anchor|slot|pattern|mask|window|signal|bus|state|timing|emit|sample|latch|rupture|bridge|seal|sensory|Primitive"
scan_rg "27. Dev/errors por modo" "dev\\(|MemoryOverflow|InputParseError|Root|Machine|Primitive|Kernel|Runtime|Security|Network|Compiler|Tooling|Inspect|Ai"
scan_rg "28. Imports/use resolver/catalog" "import_catalog|ImportCatalog|is_known_use_import_path|resolve.*import|UseDecl|use_path|nC\\.runtime|runtime\\.input"
scan_rg "29. Tests de runtime/imports/root/machine/primitive" "runtime_import|root_|machine_|primitive_|compile-pass|compile-fail|boot_serial|permisoEdad"

section "30. Ejemplos nC detectados"
echo '```txt'
find examples samples tests \
  -type f \( -name "*.n" -o -name "*.nb" -o -name "*.nm" -o -name "*.ncp" \) \
  2>/dev/null | sort | head -n 500
echo '```'

section "31. Docs/spec principales detectados"
echo '```txt'
find docs \
  -type f \( -name "*.md" -o -name "*.txt" \) \
  2>/dev/null | sort | head -n 500
echo '```'

section "32. Scripts principales detectados"
echo '```txt'
find tools -type f -name "*.sh" 2>/dev/null | sort | head -n 500
echo '```'

section "33. Resumen automático"
echo "- Informe generado en: $OUT"
echo "- Zip generado en: $ZIP"
echo "- Revisa secciones 13-16 para salud Rust."
echo "- Revisa secciones 21-29 para cierre real del lenguaje."
echo "- Si target/ pesa varios GB, no es lenguaje: es build output regenerable."
echo

cd "$(dirname "$OUT")" || exit 0
zip -q "$(basename "$ZIP")" "$(basename "$OUT")"
echo "ZIP_OK: $ZIP"
