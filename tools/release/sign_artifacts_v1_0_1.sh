#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

usage() {
  cat <<'USAGE'
Usage: sign_artifacts_v1_0_1.sh <artifact_dir> <out_dir> [--key-env <ENV_NAME>]
USAGE
}

if [ "$#" -lt 2 ]; then
  usage
  exit 2
fi

artifact_dir="$1"
out_dir="$2"
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

[ -d "$artifact_dir" ] || { echo "artifact_dir not found: $artifact_dir" >&2; exit 2; }
mkdir -p "$out_dir"

checksums_file="$out_dir/SHA256SUMS"
sig_file="$out_dir/SHA256SUMS.sig"
sbom_file="$out_dir/SBOM.ncc.v1.json"
meta_file="$out_dir/attestation.v1.json"

: > "$checksums_file"
while IFS= read -r -d '' file; do
  case "$file" in
    "$out_dir"/*) continue ;;
  esac
  rel="${file#${artifact_dir}/}"
  hash="$(sha256sum "$file" | awk '{print $1}')"
  printf "%s  %s\n" "$hash" "$rel" >> "$checksums_file"
done < <(find "$artifact_dir" -type f -print0 | sort -z)

if [ ! -s "$checksums_file" ]; then
  echo "no artifacts found to sign under $artifact_dir" >&2
  exit 2
fi

if [ -n "${!key_env:-}" ]; then
  openssl dgst -sha256 -hmac "${!key_env}" "$checksums_file" | awk '{print $2}' > "$sig_file"
else
  : > "$sig_file"
fi

"${BASH:-bash}" tools/release/generate_sbom_v1_0_1.sh "$artifact_dir" "$checksums_file" "$sbom_file"

python3 - <<'PY' "$checksums_file" "$sig_file" "$meta_file" "$key_env"
import json,sys,datetime,os
checksums_file,sig_file,meta_file,key_env = sys.argv[1:5]
count=0
with open(checksums_file,'r',encoding='utf-8') as f:
    for line in f:
        if line.strip():
            count += 1
meta={
    'schema':'ncc.release.integrity.attestation.v1',
    'version':'1.0.1',
    'series':'1.x',
    'generated_at': datetime.datetime.now(datetime.timezone.utc).replace(microsecond=0).isoformat().replace('+00:00','Z'),
    'hash_algorithm':'sha256',
    'artifact_count': count,
    'checksums_file': os.path.basename(checksums_file),
    'signature_file': os.path.basename(sig_file),
    'signing_key_env': key_env,
    'signature_present': os.path.getsize(sig_file) > 0,
}
with open(meta_file,'w',encoding='utf-8') as f:
    json.dump(meta,f,indent=2,ensure_ascii=False)
print(f"OK: integrity attestation {meta_file}")
PY

echo "OK: release integrity bundle in $out_dir"
