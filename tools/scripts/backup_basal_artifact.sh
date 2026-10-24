#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: tools/scripts/backup_basal_artifact.sh <file.nb|file.nbb> [--out-dir DIR]

Creates a byte-for-byte backup for Basal source/binary artifacts outside the
tracked example corpus by default.

Default output directory:
  target/backups/basal

The backup name keeps the original basename and appends a UTC timestamp plus
.bak. A SHA-256 sidecar is written next to the backup.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" || $# -lt 1 ]]; then
  usage
  exit 0
fi

src="$1"
shift
out_dir="target/backups/basal"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --out-dir)
      [[ $# -ge 2 ]] || { echo "--out-dir expects a directory" >&2; exit 2; }
      out_dir="$2"
      shift 2
      ;;
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

case "$src" in
  *.nb|*.nbb) ;;
  *)
    echo "refusing to back up unsupported artifact '$src'; expected .nb or .nbb" >&2
    exit 2
    ;;
esac

[[ -f "$src" ]] || { echo "source file not found: $src" >&2; exit 1; }

mkdir -p "$out_dir"
base="$(basename "$src")"
stamp="$(date -u +%Y%m%dT%H%M%SZ)"
dest="$out_dir/$base.$stamp.bak"

cp -- "$src" "$dest"
sha256sum "$dest" > "$dest.sha256"

echo "backup: $dest"
echo "sha256: $dest.sha256"
