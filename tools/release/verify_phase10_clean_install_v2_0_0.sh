#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

usage() {
  cat <<'USAGE'
Usage: verify_phase10_clean_install_v2_0_0.sh [--artifact <path>] [--report <path>] [--key-env <ENV_NAME>]
USAGE
}

ARTIFACT=""
REPORT_PATH="docs/status/F10_P1_clean_install_report.json"
KEY_ENV="NCC_RELEASE_SIGNING_KEY"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --artifact)
      shift
      [[ $# -gt 0 ]] || { echo "missing value for --artifact" >&2; exit 2; }
      ARTIFACT="$1"
      ;;
    --report)
      shift
      [[ $# -gt 0 ]] || { echo "missing value for --report" >&2; exit 2; }
      REPORT_PATH="$1"
      ;;
    --key-env)
      shift
      [[ $# -gt 0 ]] || { echo "missing value for --key-env" >&2; exit 2; }
      KEY_ENV="$1"
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

if [[ -z "$ARTIFACT" ]]; then
  HOST_TARGET="$(rustc -vV | awk '/^host:/ {print $2}')"
  ARTIFACT="var/releases/v2_0_0/phase10_p1_dist/packages/nc-v2.0.0-$HOST_TARGET.tar.gz"
fi

[[ -f "$ARTIFACT" ]] || { echo "artifact not found: $ARTIFACT" >&2; exit 2; }

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

ARTIFACT_DIR="$(cd "$(dirname "$ARTIFACT")" && pwd)"
INTEGRITY_DIR="$ARTIFACT_DIR/../integrity"
[[ -d "$INTEGRITY_DIR" ]] || { echo "integrity dir not found: $INTEGRITY_DIR" >&2; exit 2; }
INTEGRITY_DIR="$(cd "$INTEGRITY_DIR" && pwd)"
"${BASH:-bash}" tools/release/verify_artifacts_v1_0_1.sh "$ARTIFACT_DIR" "$INTEGRITY_DIR" --key-env "$KEY_ENV"

EXTRACT_DIR="$TMP_DIR/extract"
mkdir -p "$EXTRACT_DIR"
tar -C "$EXTRACT_DIR" -xzf "$ARTIFACT"

for bin in ncc nfmt nlint nbuild npkg nide nclsp ncup ncup-init; do
  [[ -x "$EXTRACT_DIR/bin/$bin" ]] || { echo "missing executable in bundle: $bin" >&2; exit 1; }
done

NCUP_HOME="$TMP_DIR/.ncup"
"$EXTRACT_DIR/bin/ncup-init" --home "$NCUP_HOME" --force --profile default --default-toolchain "stable-$(rustc -vV | awk '/^host:/ {print $2}')" >/dev/null

"$EXTRACT_DIR/bin/ncc" ncup doctor --home "$NCUP_HOME" >/dev/null
"$EXTRACT_DIR/bin/ncc" ncup verify --home "$NCUP_HOME" >/dev/null
"$EXTRACT_DIR/bin/ncc" nclsp validate examples/root_at/root_at_minimal_ok.nb --profile expert --migration-stage strict --json >/dev/null
"$EXTRACT_DIR/bin/nfmt" --help >/dev/null
"$EXTRACT_DIR/bin/nlint" --help >/dev/null
"$EXTRACT_DIR/bin/nbuild" --help >/dev/null
"$EXTRACT_DIR/bin/npkg" --help >/dev/null
"$EXTRACT_DIR/bin/nide" --help >/dev/null
"$EXTRACT_DIR/bin/nclsp" --help >/dev/null

python3 - <<'PY' "$REPORT_PATH" "$ARTIFACT" "$NCUP_HOME"
import datetime, json, pathlib, sys
report_path, artifact, ncup_home = sys.argv[1:4]
payload = {
    "schema": "nc.phase10.clean_install.report.v1",
    "version": "2.0.0",
    "status": "ok",
    "generated_at": datetime.datetime.now(datetime.timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z"),
    "artifact": artifact,
    "ncup_home": ncup_home,
    "checks": [
        "bundle bins executable",
        "distribution integrity verification",
        "ncup-init bootstrap",
        "ncup doctor",
        "ncup verify",
        "nclsp strict validation",
        "toolchain bin help surface",
    ],
}
path = pathlib.Path(report_path)
path.parent.mkdir(parents=True, exist_ok=True)
path.write_text(json.dumps(payload, indent=2, ensure_ascii=True) + "\n", encoding="utf-8")
print(f"OK: clean install report {path}")
PY

echo "[F10-P1] clean install verification OK"
