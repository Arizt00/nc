#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

usage() {
  cat <<'USAGE'
Usage: generate_sbom_v1_0_1.sh <artifact_dir> <checksums_file> <out_sbom_json>
USAGE
}

if [ "$#" -ne 3 ]; then
  usage
  exit 2
fi

artifact_dir="$1"
checksums_file="$2"
out_sbom="$3"

[ -d "$artifact_dir" ] || { echo "artifact_dir not found: $artifact_dir" >&2; exit 2; }
[ -f "$checksums_file" ] || { echo "checksums_file not found: $checksums_file" >&2; exit 2; }

python3 - <<'PY' "$artifact_dir" "$checksums_file" "$out_sbom"
import json,subprocess,sys,datetime,os
artifact_dir, checksums_file, out_sbom = sys.argv[1:4]

meta_raw = subprocess.check_output([
    'cargo','metadata','--format-version','1','--no-deps'
], text=True)
meta = json.loads(meta_raw)
packages = [
    {
        'name': p.get('name'),
        'version': p.get('version'),
        'id': p.get('id'),
        'license': p.get('license')
    }
    for p in meta.get('packages',[])
]

artifacts=[]
with open(checksums_file,'r',encoding='utf-8') as f:
    for line in f:
        line=line.strip()
        if not line:
            continue
        h, path = line.split('  ',1)
        artifacts.append({'path': path, 'sha256': h})

sbom={
    'schema':'ncc.sbom.v1',
    'version':'1.0.1',
    'series':'1.x',
    'generated_at': datetime.datetime.now(datetime.timezone.utc).replace(microsecond=0).isoformat().replace('+00:00', 'Z'),
    'workspace_root': meta.get('workspace_root'),
    'artifact_dir': os.path.abspath(artifact_dir),
    'artifact_count': len(artifacts),
    'artifacts': artifacts,
    'packages': packages
}
os.makedirs(os.path.dirname(out_sbom), exist_ok=True)
with open(out_sbom,'w',encoding='utf-8') as f:
    json.dump(sbom,f,indent=2,ensure_ascii=False)
print(f"OK: sbom {out_sbom}")
PY
