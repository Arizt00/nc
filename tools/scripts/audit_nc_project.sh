#!/usr/bin/env bash
set -u

REPORT="reports/nc_project_audit.md"
START_DIR="$(pwd)"

pass() { echo "✅ $1"; }
fail() { echo "❌ $1"; }
warn() { echo "⚠️  $1"; }

run_check() {
  local name="$1"
  shift
  echo ""
  echo "## $name" >> "$REPORT"
  echo '```txt' >> "$REPORT"
  if "$@" >> "$REPORT" 2>&1; then
    echo '```' >> "$REPORT"
    pass "$name"
    echo "**Estado:** PASS" >> "$REPORT"
  else
    echo '```' >> "$REPORT"
    fail "$name"
    echo "**Estado:** FAIL" >> "$REPORT"
  fi
}

echo "# nC Project Audit" > "$REPORT"
echo "" >> "$REPORT"
echo "- Fecha: $(date -Iseconds)" >> "$REPORT"
echo "- Directorio: $START_DIR" >> "$REPORT"
echo "" >> "$REPORT"

echo "# nC Project Audit"
echo "Directorio: $START_DIR"
echo ""

echo "## 1. Estructura base" >> "$REPORT"
echo '```txt' >> "$REPORT"
find . -maxdepth 3 -type d | sort >> "$REPORT" 2>&1
echo '```' >> "$REPORT"

run_check "Git status" git status --short
run_check "Rust version" rustc --version
run_check "Cargo version" cargo --version
run_check "Cargo metadata" cargo metadata --no-deps --format-version 1
run_check "Cargo fmt check" cargo fmt --all -- --check
run_check "Cargo test workspace" cargo test --workspace

if command -v cargo-clippy >/dev/null 2>&1; then
  run_check "Cargo clippy workspace" cargo clippy --workspace -- -D warnings
else
  echo "" >> "$REPORT"
  echo "## Cargo clippy workspace" >> "$REPORT"
  echo "⚠️ cargo-clippy no está instalado." >> "$REPORT"
  warn "cargo-clippy no está instalado"
fi

echo "" >> "$REPORT"
echo "## 2. Crates detectados" >> "$REPORT"
echo '```txt' >> "$REPORT"
find crates -maxdepth 2 -name Cargo.toml | sort >> "$REPORT" 2>&1
echo '```' >> "$REPORT"

echo "" >> "$REPORT"
echo "## 3. Binarios esperados" >> "$REPORT"
echo '```txt' >> "$REPORT"
for bin in ncc ncup ncup-init nfmt nlint nbuild npkg nide nclsp; do
  if command -v "$bin" >/dev/null 2>&1; then
    echo "$bin -> $(command -v "$bin")"
  else
    echo "$bin -> NO EN PATH"
  fi
done >> "$REPORT"
echo '```' >> "$REPORT"

run_check "ncc --help" ncc --help
run_check "ncc --version" ncc --version
run_check "ncc -V" ncc -V
run_check "ncc version" ncc version
run_check "ncup doctor" ncup doctor
run_check "ncup toolchain list available" ncup toolchain list --available
run_check "ncup which ncc" ncup which ncc

if [ -f "/home/arist/.ncup/tmp/first-run-demo/hello.n" ]; then
  run_check "ncc run first-run demo" ncc run /home/arist/.ncup/tmp/first-run-demo/hello.n
else
  echo "" >> "$REPORT"
  echo "## ncc run first-run demo" >> "$REPORT"
  echo "⚠️ No existe /home/arist/.ncup/tmp/first-run-demo/hello.n" >> "$REPORT"
fi

echo "" >> "$REPORT"
echo "## 4. Conteo de tests" >> "$REPORT"
echo '```txt' >> "$REPORT"
if command -v rg >/dev/null 2>&1; then
  echo "Tests #[test]: $(rg '#\[test\]' crates tests examples 2>/dev/null | wc -l)"
  echo "TODO/FIXME: $(rg 'TODO|FIXME|HACK|XXX' crates docs tests examples 2>/dev/null | wc -l)"
else
  echo "ripgrep no instalado; usando grep básico"
  echo "Tests #[test]: $(grep -R '#\[test\]' crates tests examples 2>/dev/null | wc -l)"
  echo "TODO/FIXME: $(grep -R -E 'TODO|FIXME|HACK|XXX' crates docs tests examples 2>/dev/null | wc -l)"
fi >> "$REPORT"
echo '```' >> "$REPORT"

echo "" >> "$REPORT"
echo "## 5. Archivos .n de ejemplo" >> "$REPORT"
echo '```txt' >> "$REPORT"
find . -type f -name "*.n" | sort | head -200 >> "$REPORT" 2>&1
echo '```' >> "$REPORT"

echo "" >> "$REPORT"
echo "## 6. Releases y toolchains locales" >> "$REPORT"
echo '```txt' >> "$REPORT"
find var releases target -maxdepth 5 \( -name "ncc" -o -name "ncc.exe" -o -name "ncup" -o -name "ncup.exe" -o -name "ncup-init" -o -name "ncup-init.exe" -o -name "manifest.v1.json" -o -name "SHA256SUMS" -o -name "*.sig" \) 2>/dev/null | sort >> "$REPORT"
echo '```' >> "$REPORT"

echo "" >> "$REPORT"
echo "## 7. Documentación detectada" >> "$REPORT"
echo '```txt' >> "$REPORT"
find docs . -maxdepth 4 \( -name "*.md" -o -name "*.txt" \) 2>/dev/null | sort | head -300 >> "$REPORT"
echo '```' >> "$REPORT"

echo "" >> "$REPORT"
echo "## 8. Matriz de madurez preliminar" >> "$REPORT"
cat >> "$REPORT" <<'EOF'

| Área | Estado preliminar | Nota |
|---|---:|---|
| CLI `ncc` | Alto | Tiene comandos canónicos y versión tras parche |
| `ncup` | Alto | Instala, resuelve toolchains, doctor e integridad |
| Lexer/parser/sema | Medio-Alto | Hay crates separados; falta medir cobertura real |
| IR/lowering/backend | Medio | Existe estructura; falta validar profundidad de emisión |
| Runtime/stdlib | Medio | Existe crate runtime; falta medir amplitud |
| Backends binarios | Medio | Hay comandos, falta probar artefactos reales |
| Tooling IDE/LSP | Medio | Binarios y docs; falta extensión funcional completa |
| Package manager | Temprano | `npkg` existe como superficie, falta madurez |
| Documentación | Medio | Hay base; falta manual estable de usuario |
| Supply chain | Alto | Checksums/firma detectan cambios |
| Cross-platform | Medio | Targets listados; falta build probado por OS |
| Comparación con grandes lenguajes | En progreso | Arquitectura prometedora; ecosistema aún temprano |

EOF

echo ""
echo "Auditoría generada en: $REPORT"
