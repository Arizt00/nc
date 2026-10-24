#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: install_ncup_linux_v2_0_0.sh --archive <nc-linux-tarball> [options]

Options:
  --prefix <dir>         Install prefix (default: /opt/nc/v2.0.0)
  --bin-dir <dir>        Symlink destination for toolchain bins (default: /usr/local/bin)
  --profile <name>       Bootstrap profile for ncup-init (default: default)
  --toolchain <id>       Default toolchain id for bootstrap (default: stable-x86_64-unknown-linux-gnu)
  --ncup-home <dir>      NCUP_HOME used during bootstrap (default: <prefix>/.ncup)
  --force                Overwrite existing prefix content
  -h|--help|help         Show this help
USAGE
}

ARCHIVE=""
PREFIX="/opt/nc/v2.0.0"
BIN_DIR="/usr/local/bin"
PROFILE="default"
DEFAULT_TOOLCHAIN="stable-x86_64-unknown-linux-gnu"
NCUP_HOME=""
FORCE=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --archive)
      shift
      [[ $# -gt 0 ]] || { echo "missing value for --archive" >&2; exit 2; }
      ARCHIVE="$1"
      ;;
    --prefix)
      shift
      [[ $# -gt 0 ]] || { echo "missing value for --prefix" >&2; exit 2; }
      PREFIX="$1"
      ;;
    --bin-dir)
      shift
      [[ $# -gt 0 ]] || { echo "missing value for --bin-dir" >&2; exit 2; }
      BIN_DIR="$1"
      ;;
    --profile)
      shift
      [[ $# -gt 0 ]] || { echo "missing value for --profile" >&2; exit 2; }
      PROFILE="$1"
      ;;
    --toolchain)
      shift
      [[ $# -gt 0 ]] || { echo "missing value for --toolchain" >&2; exit 2; }
      DEFAULT_TOOLCHAIN="$1"
      ;;
    --ncup-home)
      shift
      [[ $# -gt 0 ]] || { echo "missing value for --ncup-home" >&2; exit 2; }
      NCUP_HOME="$1"
      ;;
    --force)
      FORCE=1
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

if [[ -z "$ARCHIVE" ]]; then
  echo "--archive is required" >&2
  usage
  exit 2
fi

[[ -f "$ARCHIVE" ]] || { echo "archive not found: $ARCHIVE" >&2; exit 2; }
if [[ -z "$NCUP_HOME" ]]; then
  NCUP_HOME="$PREFIX/.ncup"
fi

if [[ -d "$PREFIX" && $FORCE -eq 0 ]]; then
  echo "prefix already exists: $PREFIX (use --force to overwrite)" >&2
  exit 2
fi

if [[ -d "$PREFIX" && $FORCE -eq 1 ]]; then
  rm -rf "$PREFIX"
fi

mkdir -p "$PREFIX" "$BIN_DIR"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

tar -C "$TMP_DIR" -xzf "$ARCHIVE"

# The archive is expected to carry a top-level toolchain/ directory.
[[ -d "$TMP_DIR/toolchain" ]] || { echo "archive is missing toolchain/ root" >&2; exit 1; }
cp -R "$TMP_DIR/toolchain" "$PREFIX/"

for bin in ncc nfmt nlint nbuild npkg nide nclsp ncup ncup-init; do
  src="$PREFIX/toolchain/bin/$bin"
  [[ -x "$src" ]] || { echo "missing executable in installed toolchain: $bin" >&2; exit 1; }
  ln -sf "$src" "$BIN_DIR/$bin"
done

export NCUP_HOME
"$PREFIX/toolchain/bin/ncup-init" \
  --home "$NCUP_HOME" \
  --force \
  --profile "$PROFILE" \
  --default-toolchain "$DEFAULT_TOOLCHAIN" >/dev/null

"$PREFIX/toolchain/bin/ncc" ncup doctor --home "$NCUP_HOME" >/dev/null

echo "OK: nC toolchain installed"
echo "  prefix: $PREFIX"
echo "  bin-dir: $BIN_DIR"
echo "  ncup-home: $NCUP_HOME"
