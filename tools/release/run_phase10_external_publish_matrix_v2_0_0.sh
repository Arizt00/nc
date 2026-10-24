#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

usage() {
  cat <<'USAGE'
Usage: run_phase10_external_publish_matrix_v2_0_0.sh [options]

Official external publication wrapper by runner role.

Options:
  --runner <name>       linux|windows|macos|all (default: linux)
  --channel <name>      stable|beta|nightly (default: stable)
  --mode <mode>         dry-run|execute (default: execute)
  --report <file>       Report path (default: docs/status/F10_external_publish_report.json)
  --allow-partial       Accept partial status
  --help                Show help
USAGE
}

RUNNER="linux"
CHANNEL="stable"
MODE="execute"
REPORT_PATH="docs/status/F10_external_publish_report.json"
ALLOW_PARTIAL=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --runner)
      shift
      [[ $# -gt 0 ]] || { echo "missing value for --runner" >&2; exit 2; }
      RUNNER="$1"
      ;;
    --channel)
      shift
      [[ $# -gt 0 ]] || { echo "missing value for --channel" >&2; exit 2; }
      CHANNEL="$1"
      ;;
    --mode)
      shift
      [[ $# -gt 0 ]] || { echo "missing value for --mode" >&2; exit 2; }
      MODE="$1"
      ;;
    --report)
      shift
      [[ $# -gt 0 ]] || { echo "missing value for --report" >&2; exit 2; }
      REPORT_PATH="$1"
      ;;
    --allow-partial)
      ALLOW_PARTIAL=1
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

case "$RUNNER" in
  linux) OPS="deb,rpm" ;;
  windows) OPS="winget" ;;
  macos) OPS="brew" ;;
  all) OPS="all" ;;
  *)
    echo "invalid --runner '$RUNNER' (expected linux|windows|macos|all)" >&2
    exit 2
    ;;
esac

CMD=(
  bash tools/release/publish_phase10_external_channels_v2_0_0.sh
  --channel "$CHANNEL"
  --mode "$MODE"
  --ops "$OPS"
  --report "$REPORT_PATH"
)

if [[ $ALLOW_PARTIAL -eq 1 ]]; then
  CMD+=(--allow-partial)
fi

echo "[external-publish-matrix] runner=$RUNNER ops=$OPS mode=$MODE channel=$CHANNEL"
"${CMD[@]}"

