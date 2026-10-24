#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

OUT_DIR="var/releases/v2_0_0/phase10_ide_dist"
JETBRAINS_OUT="$OUT_DIR/jetbrains"
VSCODE_OUT="$OUT_DIR/vscode"
GHIDRA_OUT="$OUT_DIR/ghidra"
INSTALLERS_OUT="$OUT_DIR/installers"
REPORT_PATH="docs/status/F10_P20_ide_distribution_report.json"

mkdir -p "$JETBRAINS_OUT" "$VSCODE_OUT" "$GHIDRA_OUT" "$INSTALLERS_OUT" "$(dirname "$REPORT_PATH")"

STATUS_JETBRAINS="deferred"
STATUS_VSCODE="deferred"
STATUS_GHIDRA="deferred"

# 1) JetBrains plugin artifact
if [[ -x tools/jetbrains/nc-language-plugin/gradlew ]]; then
  (
    cd tools/jetbrains/nc-language-plugin
    ./gradlew buildPlugin >/dev/null
  ) || true
elif command -v gradle >/dev/null 2>&1; then
  (
    cd tools/jetbrains/nc-language-plugin
    gradle buildPlugin >/dev/null
  ) || true
fi

JB_ZIP="$(find tools/jetbrains/nc-language-plugin/build/distributions -maxdepth 1 -name '*.zip' -type f 2>/dev/null | head -n1 || true)"
if [[ -n "$JB_ZIP" && -f "$JB_ZIP" ]]; then
  cp "$JB_ZIP" "$JETBRAINS_OUT/"
  STATUS_JETBRAINS="built"
else
  cat > "$JETBRAINS_OUT/nc-jetbrains-plugin-v2_0_0.deferred.json" <<'JSON'
{"schema":"nc.ide.jetbrains.bundle.deferred.v1","version":"2.0.0","state":"frozen","status":"deferred","reason":"gradle buildPlugin artifact unavailable on current host"}
JSON
fi

# 2) VSCode extension artifact
if command -v npx >/dev/null 2>&1; then
  (
    cd tools/vscode/nc-language
    npx --yes @vscode/vsce package --no-dependencies >/dev/null
  ) || true
fi

VSIX="$(find tools/vscode/nc-language -maxdepth 1 -name '*.vsix' -type f | head -n1 || true)"
if [[ -n "$VSIX" && -f "$VSIX" ]]; then
  cp "$VSIX" "$VSCODE_OUT/"
  STATUS_VSCODE="built"
else
  cat > "$VSCODE_OUT/nc-language-v2_0_0.deferred.json" <<'JSON'
{"schema":"nc.ide.vscode.bundle.deferred.v1","version":"2.0.0","state":"frozen","status":"deferred","reason":"vsce artifact unavailable on current host"}
JSON
fi

# 3) Ghidra extension artifact
if bash tools/ghidra/nc-ghidra-extension/build_extension_v2_0_0.sh >/dev/null; then
  STATUS_GHIDRA="built"
else
  cat > "$GHIDRA_OUT/nc-ghidra-extension-v2_0_0.deferred.json" <<'JSON'
{"schema":"nc.ide.ghidra.bundle.deferred.v1","version":"2.0.0","state":"frozen","status":"deferred","reason":"ghidra extension build failed"}
JSON
fi

# 4) Official low-level runtime installers matrix
python3 - <<'PY' "$INSTALLERS_OUT/manifest.v1.json"
import json, pathlib, sys
out = pathlib.Path(sys.argv[1])
payload = {
  "schema": "nc.ide.lowlevel.installers.v1",
  "version": "2.0.0",
  "state": "frozen",
  "policy": {
    "sources": ["official-site", "github-releases"],
    "low_level_runtime_required_for": ["jetbrains-low-level", "vscode-low-level", "ghidra"],
    "windows_binary_pattern": "ncc.{ide}.windows.exe"
  },
  "installers": [
    {"ide": "jetbrains", "os": "windows", "artifact": "ncc.jetbrains.windows.exe", "status": "planned"},
    {"ide": "intellij", "os": "windows", "artifact": "ncc.intellij.windows.exe", "status": "planned"},
    {"ide": "vscode", "os": "windows", "artifact": "ncc.vscode.windows.exe", "status": "planned"},
    {"ide": "ghidra", "os": "windows", "artifact": "ncc.ghidra.windows.exe", "status": "planned"},
    {"ide": "jetbrains", "os": "linux", "artifact": "ncc.jetbrains.linux.sh", "status": "planned"},
    {"ide": "intellij", "os": "linux", "artifact": "ncc.intellij.linux.sh", "status": "planned"},
    {"ide": "vscode", "os": "linux", "artifact": "ncc.vscode.linux.sh", "status": "planned"},
    {"ide": "ghidra", "os": "linux", "artifact": "ncc.ghidra.linux.sh", "status": "planned"},
    {"ide": "jetbrains", "os": "macos", "artifact": "ncc.jetbrains.macos.pkg", "status": "planned"},
    {"ide": "intellij", "os": "macos", "artifact": "ncc.intellij.macos.pkg", "status": "planned"},
    {"ide": "vscode", "os": "macos", "artifact": "ncc.vscode.macos.pkg", "status": "planned"},
    {"ide": "ghidra", "os": "macos", "artifact": "ncc.ghidra.macos.pkg", "status": "planned"}
  ]
}
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(json.dumps(payload, indent=2, ensure_ascii=True) + "\n", encoding="utf-8")
print(f"OK: ide installers manifest {out}")
PY

python3 - <<'PY' "$REPORT_PATH" "$STATUS_JETBRAINS" "$STATUS_VSCODE" "$STATUS_GHIDRA" "$OUT_DIR"
import datetime, json, pathlib, sys
report, jb, vs, gh, out_dir = sys.argv[1:6]
status = "ok"
if "deferred" in (jb, vs, gh):
    status = "partial"
payload = {
  "schema": "nc.phase10.ide.distribution.report.v1",
  "version": "2.0.0",
  "state": "frozen",
  "generated_at": datetime.datetime.now(datetime.timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z"),
  "status": status,
  "out_dir": out_dir,
  "ide_bundles": {
    "jetbrains": jb,
    "vscode": vs,
    "ghidra": gh
  }
}
path = pathlib.Path(report)
path.parent.mkdir(parents=True, exist_ok=True)
path.write_text(json.dumps(payload, indent=2, ensure_ascii=True) + "\n", encoding="utf-8")
print(f"OK: ide distribution report {path}")
PY

echo "OK: phase10 IDE distribution -> $OUT_DIR"
