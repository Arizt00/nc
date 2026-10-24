#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

usage() {
  cat <<'USAGE'
Usage: verify_artifacts_v1_0_1.sh <artifact_dir> <integrity_dir> [--key-env <ENV_NAME>]
USAGE
}

if [ "$#" -lt 2 ]; then
  usage
  exit 2
fi

artifact_dir="$1"
integrity_dir="$2"
shift 2

key_env="NCC_RELEASE_SIGNING_KEY"
while [ "$#" -gt 0 ]; do
  case "$1" in
    --key-env)
      shift
      [ "$#" -gt 0 ] || { echo "missing value for --key-env" >&2; exit 2; }
      key_env="$1"
      ;;
    -h|--help|help)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage
      exit 2
      ;;
  esac
  shift || true
 done

checksums_file="$integrity_dir/SHA256SUMS"
sig_file="$integrity_dir/SHA256SUMS.sig"
sbom_file="$integrity_dir/SBOM.ncc.v1.json"
meta_file="$integrity_dir/attestation.v1.json"

[ -d "$artifact_dir" ] || { echo "artifact_dir not found: $artifact_dir" >&2; exit 2; }
[ -f "$checksums_file" ] || { echo "missing checksums file: $checksums_file" >&2; exit 2; }
[ -f "$sig_file" ] || { echo "missing signature file: $sig_file" >&2; exit 2; }
[ -f "$sbom_file" ] || { echo "missing sbom file: $sbom_file" >&2; exit 2; }
[ -f "$meta_file" ] || { echo "missing attestation file: $meta_file" >&2; exit 2; }

( cd "$artifact_dir" && sha256sum -c "$checksums_file" )

if [ -s "$sig_file" ]; then
  [ -n "${!key_env:-}" ] || { echo "signature exists but key env '$key_env' is empty" >&2; exit 2; }
  expected="$(cat "$sig_file")"
  actual="$(openssl dgst -sha256 -hmac "${!key_env}" "$checksums_file" | awk '{print $2}')"
  [ "$expected" = "$actual" ] || { echo "signature mismatch for $checksums_file" >&2; exit 2; }
fi

python3 - <<'PY' "$sbom_file" "$checksums_file" "$meta_file"
import json,sys
sbom_path, checksums_path, meta_path = sys.argv[1:4]
with open(sbom_path,'r',encoding='utf-8') as f:
    sbom=json.load(f)
if sbom.get('schema') != 'ncc.sbom.v1':
    raise SystemExit('invalid sbom schema')
if not isinstance(sbom.get('artifacts'), list) or not sbom['artifacts']:
    raise SystemExit('sbom must include artifacts list')

line_count=0
with open(checksums_path,'r',encoding='utf-8') as f:
    for line in f:
        if line.strip():
            line_count += 1
if sbom.get('artifact_count') != line_count:
    raise SystemExit('sbom artifact_count does not match checksums')

with open(meta_path,'r',encoding='utf-8') as f:
    meta=json.load(f)
if meta.get('schema') != 'ncc.release.integrity.attestation.v1':
    raise SystemExit('invalid attestation schema')
if meta.get('artifact_count') != line_count:
    raise SystemExit('attestation artifact_count does not match checksums')
print('OK: SBOM + attestation validated')
PY

echo "OK: artifact integrity verification passed"
