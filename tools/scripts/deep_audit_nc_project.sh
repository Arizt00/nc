#!/usr/bin/env bash

set +e

OUT="reports/nc_deep_project_audit_$(date +%Y%m%d_%H%M%S).md"

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

echo "# nC / ncc — Auditoría profunda del proyecto"
echo
echo "- Fecha: $(date -Is)"
echo "- Directorio: $(pwd)"
echo "- Usuario: $(whoami)"
echo "- Sistema: $(uname -a)"
echo

section "0. Comprobación de herramientas disponibles"
for tool in git rustc cargo rustfmt clippy-driver rg tree find awk sed grep wc; do
    if command -v "$tool" >/dev/null 2>&1; then
        echo "- ✅ $tool -> $(command -v "$tool")"
    else
        echo "- ❌ $tool no encontrado"
    fi
done

run_cmd "1. Estado Git" git status --short --branch
run_cmd "2. Últimos commits" git log --oneline --decorate --graph -n 30
run_cmd "3. Remotos Git" git remote -v
run_cmd "4. Rust version" rustc --version
run_cmd "5. Cargo version" cargo --version
run_cmd "6. Rustup toolchain activa" rustup show active-toolchain
run_cmd "7. Cargo metadata workspace" cargo metadata --no-deps --format-version 1
run_cmd "8. Cargo tree workspace" cargo tree --workspace

section "9. Estructura de directorios"
if command -v tree >/dev/null 2>&1; then
    tree -a -L 5 -I "target|.git|node_modules|.idea|.vscode"
else
    find . \
        -path "./target" -prune -o \
        -path "./.git" -prune -o \
        -path "./node_modules" -prune -o \
        -type d -maxdepth 5 -print | sort
fi

section "10. Archivos principales detectados"
find . \
    -path "./target" -prune -o \
    -path "./.git" -prune -o \
    -type f \( \
        -name "*.rs" -o \
        -name "*.n" -o \
        -name "*.toml" -o \
        -name "*.md" -o \
        -name "*.json" -o \
        -name "*.yaml" -o \
        -name "*.yml" -o \
        -name "*.sh" \
    \) -print | sort

section "11. Conteo aproximado por extensión"
find . \
    -path "./target" -prune -o \
    -path "./.git" -prune -o \
    -type f -print \
| sed 's|.*\.||' \
| sort \
| uniq -c \
| sort -nr

section "12. Líneas de código aproximadas"
find . \
    -path "./target" -prune -o \
    -path "./.git" -prune -o \
    -type f \( -name "*.rs" -o -name "*.n" -o -name "*.md" -o -name "*.toml" -o -name "*.sh" \) -print0 \
| xargs -0 wc -l \
| sort -nr \
| head -n 80

run_cmd "13. Cargo fmt check" cargo fmt --all -- --check
run_cmd "14. Cargo check workspace" cargo check --workspace --all-targets
run_cmd "15. Cargo test workspace" cargo test --workspace --all-targets
run_cmd "16. Cargo clippy workspace" cargo clippy --workspace --all-targets -- -D warnings

section "17. Binarios generados en target/debug"
find target/debug -maxdepth 2 -type f -perm -111 2>/dev/null | sort || true

section "18. Comandos ncc / ncup si existen"
if [ -x "./target/debug/ncc" ]; then
    run_cmd "target/debug/ncc --help" ./target/debug/ncc --help
    run_cmd "target/debug/ncc --version" ./target/debug/ncc --version
fi

if [ -x "./target/debug/ncc-cli" ]; then
    run_cmd "target/debug/ncc-cli --help" ./target/debug/ncc-cli --help
fi

if command -v ncc >/dev/null 2>&1; then
    run_cmd "ncc --help" ncc --help
    run_cmd "ncc --version" ncc --version
    run_cmd "ncc -V" ncc -V
fi

if command -v ncup >/dev/null 2>&1; then
    run_cmd "ncup doctor" ncup doctor
    run_cmd "ncup toolchain list available" ncup toolchain list available
    run_cmd "ncup which ncc" ncup which ncc
fi

section "19. Búsqueda de TODO/FIXME/panic/unwrap/expect"
if command -v rg >/dev/null 2>&1; then
    rg -n --hidden -g '!target' -g '!.git' -g '!node_modules' "TODO|FIXME|HACK|panic!|unwrap\(|expect\(|todo!\(|unimplemented!\(" . || true
else
    grep -RInE "TODO|FIXME|HACK|panic!|unwrap\(|expect\(|todo!\(|unimplemented!\(" . --exclude-dir=target --exclude-dir=.git || true
fi

section "20. Búsqueda de piezas nC: lexer/parser/AST/sema/lowering/nIR/backend"
if command -v rg >/dev/null 2>&1; then
    rg -n --hidden -g '!target' -g '!.git' -g '!node_modules' "lexer|Lexer|TokenKind|parser|Parser|AST|Ast|semantic|sema|lowering|nIR|nir|backend|diagnostic|Diagnostic|span|Span" . || true
else
    grep -RInE "lexer|Lexer|TokenKind|parser|Parser|AST|Ast|semantic|sema|lowering|nIR|nir|backend|diagnostic|Diagnostic|span|Span" . --exclude-dir=target --exclude-dir=.git || true
fi

section "21. Búsqueda de superficie nC: modos, runtime, print, machine, primitive"
if command -v rg >/dev/null 2>&1; then
    rg -n --hidden -g '!target' -g '!.git' -g '!node_modules' "\[@\]|\[@runtime\]|\[@system\]|\[@kernel\]|\[@machine\]|primitive\[@\]|runtime\.|use nC\.runtime|print\.sensory|print\.mmio|print\.port|print\.dump|print\.addr|print\.stable|readInt|readLine|->>|!>|dev\(" . || true
else
    grep -RInE "\[@\]|\[@runtime\]|\[@system\]|\[@kernel\]|\[@machine\]|primitive\[@\]|runtime\.|use nC\.runtime|print\.sensory|print\.mmio|print\.port|print\.dump|print\.addr|print\.stable|readInt|readLine|->>|!>|dev\(" . --exclude-dir=target --exclude-dir=.git || true
fi

section "22. Búsqueda de memoria/hardware: ptr, addr, MMIO, port, volatile, atomic, unsafe"
if command -v rg >/dev/null 2>&1; then
    rg -n --hidden -g '!target' -g '!.git' -g '!node_modules' "ptr<|memRegion|addr|MMIO|mmio|port|volatile|atomic|fence|interrupt|boot|kernel|no_std|unsafe|asm!|global_asm|linker|ABI|target" . || true
else
    grep -RInE "ptr<|memRegion|addr|MMIO|mmio|port|volatile|atomic|fence|interrupt|boot|kernel|no_std|unsafe|asm!|global_asm|linker|ABI|target" . --exclude-dir=target --exclude-dir=.git || true
fi

section "23. Samples .n detectados"
find . \
    -path "./target" -prune -o \
    -path "./.git" -prune -o \
    -type f -name "*.n" -print | sort

section "24. Scripts detectados"
find tools scripts .github -type f 2>/dev/null | sort || true

section "25. Resumen final automático"
echo "- Informe generado en: $OUT"
echo "- Revisa especialmente las secciones 13, 14, 15 y 16 para saber si el compilador está sano."
echo "- Revisa las secciones 20, 21 y 22 para ver cuánto hay realmente de lexer/parser/sema/nIR/backend/hardware."
echo
echo "FIN"
