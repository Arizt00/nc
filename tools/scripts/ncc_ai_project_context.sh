#!/usr/bin/env bash
set -u
set -o pipefail

ROOT="${1:-$(pwd)}"
cd "$ROOT" || {
  echo "ERROR: no puedo entrar en ROOT=$ROOT"
  exit 1
}

TS="$(date +%Y%m%d_%H%M%S)"
OUT_DIR="reports/ai_project_context"
JSON_DIR="$OUT_DIR/json_$TS"
LOG_DIR="$OUT_DIR/logs_$TS"
OUT="$OUT_DIR/ncc_ai_context_$TS.md"
PROMPT="$OUT_DIR/ncc_ai_prompt_$TS.md"

mkdir -p "$OUT_DIR" "$JSON_DIR" "$LOG_DIR"

hr() {
  printf '\n---\n\n' >> "$OUT"
}

section() {
  local title="$1"
  hr
  printf '# %s\n\n' "$title" >> "$OUT"
}

subsection() {
  local title="$1"
  printf '\n## %s\n\n' "$title" >> "$OUT"
}

run_capture() {
  local name="$1"
  shift

  local safe_name
  safe_name="$(echo "$name" | tr ' /:' '___' | tr -cd '[:alnum:]_-.')"
  local log="$LOG_DIR/${safe_name}.log"

  {
    echo "\$ $*"
    echo
    "$@"
  } >"$log" 2>&1
  local code=$?

  {
    echo "### $name"
    echo
    echo '```txt'
    sed -n '1,220p' "$log"
    local lines
    lines="$(wc -l < "$log" | tr -d ' ')"
    if [ "$lines" -gt 220 ]; then
      echo
      echo "... salida truncada en informe. Log completo: $log"
    fi
    echo '```'
    echo
    echo "exit_code: $code"
    echo
  } >> "$OUT"

  return 0
}

run_optional() {
  local name="$1"
  shift
  if command -v "$1" >/dev/null 2>&1; then
    run_capture "$name" "$@"
  else
    {
      echo "### $name"
      echo
      echo "No disponible: $1"
      echo
    } >> "$OUT"
  fi
}

cat > "$OUT" <<EOF
# ncc AI Project Context

- timestamp: $TS
- root: $ROOT
- host: $(hostname 2>/dev/null || echo unknown)
- user: $(whoami 2>/dev/null || echo unknown)

Este informe está pensado para pegarlo o subirlo a ChatGPT y permitir una lectura global del estado real del proyecto nC/ncc.

EOF

section "Resumen rápido"

{
  echo '```txt'
  echo "PWD: $(pwd)"
  echo "Fecha: $(date)"
  echo
  echo "Top-level:"
  find . -maxdepth 1 -mindepth 1 \
    -not -path './target' \
    -not -path './.git' \
    -not -path './reports' \
    -printf '%f\n' 2>/dev/null | sort
  echo '```'
} >> "$OUT"

section "Git"
run_optional "git status" git status --short
run_optional "git branch" git branch --show-current
run_optional "git recent commits" git log --oneline -n 12
run_optional "git diff stat" git diff --stat

section "Rust workspace / Cargo"
if [ -f Cargo.toml ]; then
  run_optional "cargo metadata" cargo metadata --no-deps --format-version 1
  run_optional "cargo tree short" cargo tree --depth 2
else
  echo "No hay Cargo.toml en la raíz detectada." >> "$OUT"
fi

section "Binarios ncc disponibles"
{
  echo '```txt'
  for b in ncc nfmt nlint nbuild npkg nide nclsp ncup ncup-init nwal ghor; do
    printf "%-12s " "$b"
    if command -v "$b" >/dev/null 2>&1; then
      echo "FOUND -> $(command -v "$b")"
    else
      echo "MISSING"
    fi
  done
  echo '```'
} >> "$OUT"

subsection "Ayuda de binarios principales"
for b in ncc nfmt nlint nbuild npkg nide nwal; do
  if command -v "$b" >/dev/null 2>&1; then
    run_capture "$b --help" "$b" --help
  fi
done

section "Árbol del proyecto"
{
  echo '```txt'
  if command -v tree >/dev/null 2>&1; then
    tree -a -L 4 \
      -I 'target|.git|reports|node_modules|__pycache__|*.rlib|*.rmeta' .
  else
    find . -maxdepth 4 \
      -not -path './target/*' \
      -not -path './.git/*' \
      -not -path './reports/*' \
      | sort
  fi
  echo '```'
} >> "$OUT"

section "Inventario de archivos"
python3 - <<'PY' "$ROOT" "$JSON_DIR/file_inventory.json" >> "$OUT"
import json, os, sys
from pathlib import Path
root = Path(sys.argv[1]).resolve()
out = Path(sys.argv[2])

skip_dirs = {".git", "target", "reports", "node_modules", "__pycache__"}
counts = {}
samples = {}
total = 0

for p in root.rglob("*"):
    if any(part in skip_dirs for part in p.parts):
        continue
    if p.is_file():
        total += 1
        ext = p.suffix.lower() or "<no_ext>"
        counts[ext] = counts.get(ext, 0) + 1
        samples.setdefault(ext, [])
        if len(samples[ext]) < 20:
            samples[ext].append(str(p.relative_to(root)))

data = {
    "total_files": total,
    "extension_counts": dict(sorted(counts.items(), key=lambda x: (-x[1], x[0]))),
    "samples": samples,
}
out.write_text(json.dumps(data, indent=2, ensure_ascii=False), encoding="utf-8")

print("```json")
print(json.dumps(data, indent=2, ensure_ascii=False))
print("```")
PY

section "Análisis nC/ncc de fuentes"
python3 - <<'PY' "$ROOT" "$JSON_DIR/nc_source_scan.json" >> "$OUT"
import json, re, sys
from pathlib import Path

root = Path(sys.argv[1]).resolve()
out = Path(sys.argv[2])

skip_dirs = {".git", "target", "reports", "node_modules", "__pycache__"}

exts = {".n", ".ncp", ".nm", ".nb", ".nbb"}
mode_re = re.compile(r'([A-Za-z_][A-Za-z0-9_]*)\s*\[\s*@([A-Za-z_][A-Za-z0-9_]*)\s*\]')
primitive_re = re.compile(r'([A-Za-z_][A-Za-z0-9_]*)\s*primitive\s*\[@\]')
use_re = re.compile(r'use\s*<([^>]+)>;')
project_use_re = re.compile(r'use\s*<\s*project\s*:\s*([^>]+)>;')
main_ref_re = re.compile(r'ref\s+main\s*->>\s*@fn\s*\(([^)]+)\)')
to_main_re = re.compile(r'\bto\s+main\b')
fn_re = re.compile(r'([A-Za-z_][A-Za-z0-9_]*)\s*\([^)]*\)\s*->\s*([A-Za-z_][A-Za-z0-9_<>, ]*)')
type_re = re.compile(r'^\s*([A-Z][A-Za-z0-9_]*)\s*\{', re.M)

files = []
mode_counts = {}
imports = {}
project_imports = {}
main_files = []
containers = {}
functions = {}
types = {}
interesting_calls = {
    "alloc": 0, "free": 0, "ptrRead": 0, "ptrWrite": 0,
    "regionCreate": 0, "regionRead": 0, "regionWrite": 0,
    "listFromArray": 0, "listClone": 0, "listContains": 0,
    "queue": 0, "fswrite": 0, "fsread": 0, "fsappend": 0, "fsync": 0,
    "tcp": 0, "foreign": 0, "syscall": 0, "emit-llvm": 0
}

for p in root.rglob("*"):
    if any(part in skip_dirs for part in p.parts):
        continue
    if not p.is_file() or p.suffix.lower() not in exts:
        continue
    rel = str(p.relative_to(root))
    try:
        text = p.read_text(encoding="utf-8", errors="replace")
    except Exception as e:
        files.append({"file": rel, "error": str(e)})
        continue

    found_modes = []
    for name, mode in mode_re.findall(text):
        found_modes.append({"container": name, "mode": mode})
        mode_counts[mode] = mode_counts.get(mode, 0) + 1
        containers.setdefault(name, []).append(rel)

    if "primitive[@" in text:
        mode_counts["primitive"] = mode_counts.get("primitive", 0) + 1

    use_list = [u.strip() for u in use_re.findall(text)]
    for u in use_list:
        imports[u] = imports.get(u, 0) + 1

    proj = []
    for group in project_use_re.findall(text):
        mods = [x.strip() for x in group.split(",") if x.strip()]
        proj.extend(mods)
        for m in mods:
            project_imports[m] = project_imports.get(m, 0) + 1

    main_refs = main_ref_re.findall(text)
    has_to_main = bool(to_main_re.search(text))
    if main_refs or has_to_main:
        main_files.append({
            "file": rel,
            "ref_main": main_refs,
            "to_main": has_to_main
        })

    fns = fn_re.findall(text)
    if fns:
        functions[rel] = [{"name": a, "return": b.strip()} for a, b in fns[:80]]

    tps = type_re.findall(text)
    if tps:
        types[rel] = tps[:80]

    for k in interesting_calls:
        interesting_calls[k] += text.count(k)

    files.append({
        "file": rel,
        "bytes": len(text.encode("utf-8")),
        "lines": text.count("\n") + 1,
        "modes": found_modes,
        "project_imports": proj,
        "main_refs": main_refs,
        "to_main": has_to_main,
        "uses": use_list[:40],
    })

data = {
    "source_file_count": len(files),
    "mode_counts": dict(sorted(mode_counts.items())),
    "main_files": main_files,
    "project_imports": dict(sorted(project_imports.items())),
    "top_imports": sorted(imports.items(), key=lambda x: (-x[1], x[0]))[:80],
    "containers": {k:v for k,v in sorted(containers.items())},
    "interesting_call_counts": interesting_calls,
    "functions_by_file_sample": functions,
    "types_by_file_sample": types,
    "files": files,
}
out.write_text(json.dumps(data, indent=2, ensure_ascii=False), encoding="utf-8")

print("```json")
compact = dict(data)
compact["files"] = files[:80]
print(json.dumps(compact, indent=2, ensure_ascii=False))
print("```")
PY

section "Project Bridge / workspace"
for f in bridge.json ncc.package.json ncc.lock; do
  if [ -f "$f" ]; then
    subsection "$f"
    {
      echo '```json'
      sed -n '1,220p' "$f"
      echo '```'
    } >> "$OUT"
  fi
done

if command -v ncc >/dev/null 2>&1; then
  run_capture "ncc project doctor" ncc project doctor
  run_capture "ncc project graph" ncc project graph
fi

section "Examples oficiales"
{
  echo '```txt'
  if [ -d examples ]; then
    find examples \
      -type f \( -name '*.n' -o -name '*.ncp' -o -name '*.nm' \) \
      -not -path '*/target/*' \
      | sort \
      | sed -n '1,300p'
  else
    echo "No existe examples/"
  fi
  echo '```'
} >> "$OUT"

section "Ejemplos de mayor calibre detectados"
python3 - <<'PY' "$ROOT" "$JSON_DIR/high_caliber_examples.json" >> "$OUT"
import json, sys
from pathlib import Path

root = Path(sys.argv[1]).resolve()
out = Path(sys.argv[2])
skip = {".git", "target", "reports", "node_modules", "__pycache__"}
exts = {".n", ".ncp", ".nm"}

keywords = {
    "ptr": 3,
    "memRegion": 3,
    "heap": 3,
    "region": 2,
    "list": 2,
    "queue": 2,
    "fswrite": 2,
    "fsread": 2,
    "fsync": 3,
    "tcp": 3,
    "foreign": 3,
    "Project": 2,
    "use <project:": 3,
    "primitive[@": 5,
    "[@machine]": 5,
    "emit": 2,
    "LLVM": 3,
    "WAL": 4,
    "wal": 4,
    "syscall": 3,
    "ptrRead": 3,
    "ptrWrite": 3,
    "regionRead": 3,
    "regionWrite": 3,
    "array": 1,
    "matrix": 1,
}
rows = []
for p in root.rglob("*"):
    if any(part in skip for part in p.parts):
        continue
    if not p.is_file() or p.suffix.lower() not in exts:
        continue
    try:
        text = p.read_text(encoding="utf-8", errors="replace")
    except Exception:
        continue
    score = 0
    hits = {}
    for k, w in keywords.items():
        c = text.count(k)
        if c:
            hits[k] = c
            score += c * w
    if score:
        rows.append({
            "file": str(p.relative_to(root)),
            "score": score,
            "lines": text.count("\n") + 1,
            "hits": hits
        })
rows.sort(key=lambda x: (-x["score"], x["file"]))
out.write_text(json.dumps(rows[:100], indent=2, ensure_ascii=False), encoding="utf-8")
print("```json")
print(json.dumps(rows[:40], indent=2, ensure_ascii=False))
print("```")
PY

section "Diagnósticos y tests rápidos"
if [ -f Cargo.toml ]; then
  run_capture "cargo test --workspace --no-fail-fast" cargo test --workspace --no-fail-fast
  run_capture "cargo build --workspace --bins" cargo build --workspace --bins
fi

section "Comandos ncc doctor/check disponibles"
if command -v ncc >/dev/null 2>&1; then
  run_capture "ncc compiler doctor" ncc compiler doctor
  run_capture "ncc runtime doctor" ncc runtime doctor
  run_capture "ncc mct manifest json" ncc mct manifest --json
  run_capture "ncc imports doctor json" ncc imports doctor --json
else
  echo "ncc no está instalado en PATH." >> "$OUT"
fi

section "Archivos generados"
{
  echo '```txt'
  echo "Informe principal: $OUT"
  echo "Prompt IA:         $PROMPT"
  echo "JSON dir:          $JSON_DIR"
  echo "Logs dir:          $LOG_DIR"
  echo '```'
} >> "$OUT"

cat > "$PROMPT" <<EOF
# Contexto para ChatGPT sobre mi proyecto nC/ncc

He generado este informe automático del proyecto:

- Informe: $OUT
- JSON: $JSON_DIR
- Logs: $LOG_DIR

Quiero que analices el estado real de ncc y me digas:
1. qué partes están cerradas,
2. qué partes están incompletas,
3. qué ejemplos/programas avanzados puedo crear ahora,
4. qué ejemplos no conviene hacer todavía,
5. qué debería preparar para beta pública,
6. qué riesgos técnicos ves.

EOF

echo "OK: informe generado"
echo "$OUT"
echo "$PROMPT"
