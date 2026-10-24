#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT_DIR"

OUT_DIR="var/releases/v2_0_0/phase10_ide_dist/ghidra"
EXT_DIR="tools/ghidra/nc-ghidra-extension"
ZIP_PATH="$OUT_DIR/nc-ghidra-extension-v2_0_0.zip"

mkdir -p "$OUT_DIR"
rm -f "$ZIP_PATH"

(
  cd "$EXT_DIR"
  zip -qr "$ROOT_DIR/$ZIP_PATH" README.md extension.properties Module.manifest bin ghidra_scripts
)

python3 - <<'PY' "$ZIP_PATH"
import json, hashlib, pathlib, sys
zip_path = pathlib.Path(sys.argv[1])
sha = hashlib.sha256(zip_path.read_bytes()).hexdigest()
manifest = {
  "schema": "nc.ghidra.extension.bundle.v1",
  "version": "2.0.0",
  "state": "frozen",
  "artifact": str(zip_path),
  "sha256": sha
}
out = zip_path.parent / "ghidra_extension_manifest.v1.json"
out.write_text(json.dumps(manifest, indent=2, ensure_ascii=True) + "\n", encoding="utf-8")
print(f"OK: ghidra extension manifest {out}")
PY

echo "OK: ghidra extension bundle -> $ZIP_PATH"
