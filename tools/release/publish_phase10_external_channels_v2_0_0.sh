#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

usage() {
  cat <<'USAGE'
Usage: publish_phase10_external_channels_v2_0_0.sh [options]

Official external publication orchestrator for v2.0.0 artifacts.

Options:
  --dist-root <dir>         Distribution root (default: var/releases/v2_0_0/phase10_p3_dist)
  --channel <name>          Release channel (stable|beta|nightly; default: stable)
  --ops <csv>               Operations subset: all|winget,brew,deb,rpm (default: all)
  --mode <mode>             dry-run|execute (default: dry-run)
  --report <file>           JSON report path (default: docs/status/F10_external_publish_report.json)
  --allow-partial           Do not fail when some adapters are deferred/skipped
  --help                    Show help

Environment (execute mode adapters):
  NC_WINGET_SUBMIT_CMD      Optional full command used to submit winget manifests
  NC_BREW_TAP_PUBLISH_CMD   Optional full command used to publish brew tap updates
  NC_DEB_PUBLISH_CMD        Optional full command used to publish deb package
  NC_RPM_PUBLISH_CMD        Optional full command used to publish rpm package
USAGE
}

DIST_ROOT="var/releases/v2_0_0/phase10_p3_dist"
CHANNEL="stable"
MODE="dry-run"
REPORT_PATH="docs/status/F10_external_publish_report.json"
ALLOW_PARTIAL=0
OPS="all"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dist-root)
      shift
      [[ $# -gt 0 ]] || { echo "missing value for --dist-root" >&2; exit 2; }
      DIST_ROOT="$1"
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
    --ops)
      shift
      [[ $# -gt 0 ]] || { echo "missing value for --ops" >&2; exit 2; }
      OPS="$1"
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

if [[ "$MODE" != "dry-run" && "$MODE" != "execute" ]]; then
  echo "invalid --mode '$MODE' (expected dry-run|execute)" >&2
  exit 2
fi

case "$CHANNEL" in
  stable|beta|nightly) ;;
  *)
    echo "invalid --channel '$CHANNEL' (expected stable|beta|nightly)" >&2
    exit 2
    ;;
esac

if [[ "$OPS" != "all" ]]; then
  IFS=',' read -r -a _ops <<< "$OPS"
  for op in "${_ops[@]}"; do
    case "$(echo "$op" | tr '[:upper:]' '[:lower:]' | xargs)" in
      winget|brew|deb|rpm) ;;
      *)
        echo "invalid --ops selector '$op' (expected winget|brew|deb|rpm)" >&2
        exit 2
        ;;
    esac
  done
fi

DIST_PATH="$ROOT_DIR/$DIST_ROOT"
if [[ ! -d "$DIST_PATH/packages" ]]; then
  echo "distribution packages not found: $DIST_PATH/packages" >&2
  exit 1
fi
if [[ ! -d "$DIST_PATH/integrity" ]]; then
  echo "integrity bundle not found: $DIST_PATH/integrity" >&2
  exit 1
fi

TMP_DIR="$(mktemp -d)"
PLAN_JSON="$TMP_DIR/plan.json"
RUN_LOG="$TMP_DIR/run.log"

# Build deterministic publication plan from produced artifacts.
python3 - <<'PY' "$DIST_PATH" "$CHANNEL" "$PLAN_JSON" "$OPS"
import json
import pathlib
import sys

dist = pathlib.Path(sys.argv[1])
channel = sys.argv[2]
out = pathlib.Path(sys.argv[3])
ops_selector = sys.argv[4].strip().lower()

packages = dist / "packages"
integrity = dist / "integrity"

windows_pkg = next((p for p in (packages / "windows").glob("nc-v2.0.0-*-portable.zip")), None)
windows_winget = packages / "windows" / "winget" / "nc" / "2.0.0"
windows_integrity = integrity / "windows" / "SHA256SUMS"

linux_tar = next((p for p in (packages / "linux").glob("nc-v2.0.0-*.tar.gz")), None)
linux_deb = next((p for p in (packages / "linux").glob("*.deb")), None)
linux_rpm = next((p for p in (packages / "linux").glob("*.rpm")), None)
linux_integrity = integrity / "linux" / "SHA256SUMS"

mac_tar = next((p for p in (packages / "macos").glob("nc-v2.0.0-*.tar.gz")), None)
mac_brew_formula = packages / "macos" / "brew-tap" / "Formula" / "nc-toolchain.rb"
mac_integrity = integrity / "macos" / "SHA256SUMS"

ops = [
    {
        "id": "winget",
        "required": [
            str(windows_pkg) if windows_pkg else "",
            str(windows_winget / "nc.installer.yaml"),
            str(windows_winget / "nc.defaultLocale.yaml"),
            str(windows_winget / "nc.yaml"),
            str(windows_integrity),
        ],
        "publish_hint": "wingetcreate submit <manifest-dir>",
    },
    {
        "id": "brew",
        "required": [
            str(mac_tar) if mac_tar else "",
            str(mac_brew_formula),
            str(mac_integrity),
        ],
        "publish_hint": "git push tap && brew test-bot",
    },
    {
        "id": "deb",
        "required": [
            str(linux_deb) if linux_deb else "",
            str(linux_integrity),
        ],
        "publish_hint": "reprepro includedeb <suite> <deb>",
    },
    {
        "id": "rpm",
        "required": [
            str(linux_rpm) if linux_rpm else "",
            str(linux_integrity),
        ],
        "publish_hint": "createrepo_c + rsync/s3 sync",
    },
]

if ops_selector != "all":
    selected = {part.strip().lower() for part in ops_selector.split(",") if part.strip()}
    ops = [op for op in ops if op["id"] in selected]

for op in ops:
    missing = []
    for raw in op["required"]:
        if not raw:
            missing.append("missing-artifact")
            continue
        p = pathlib.Path(raw)
        if not p.exists() or not p.is_file() or p.stat().st_size == 0:
            missing.append(str(p))
    op["missing"] = missing
    op["ready"] = len(missing) == 0

payload = {
    "schema": "nc.phase10.external.publish.plan.v1",
    "version": "2.0.0",
    "state": "frozen",
    "channel": channel,
    "distribution_root": str(dist),
    "operations": ops,
}
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(json.dumps(payload, indent=2, ensure_ascii=True) + "\n", encoding="utf-8")
print(out)
PY

run_adapter() {
  local op_id="$1"
  local cmd="$2"
  if [[ -z "$cmd" ]]; then
    printf "[%s] skipped (no adapter command configured)\n" "$op_id" | tee -a "$RUN_LOG"
    return 10
  fi
  printf "[%s] execute: %s\n" "$op_id" "$cmd" | tee -a "$RUN_LOG"
  bash -lc "$cmd" >>"$RUN_LOG" 2>&1
}

PY_STATUS="$(python3 - <<'PY' "$PLAN_JSON" "$MODE" "$RUN_LOG"
import datetime
import json
import os
import pathlib
import subprocess
import sys

plan_path = pathlib.Path(sys.argv[1])
mode = sys.argv[2]
run_log = pathlib.Path(sys.argv[3])
plan = json.loads(plan_path.read_text(encoding="utf-8"))

adapter_env = {
    "winget": "NC_WINGET_SUBMIT_CMD",
    "brew": "NC_BREW_TAP_PUBLISH_CMD",
    "deb": "NC_DEB_PUBLISH_CMD",
    "rpm": "NC_RPM_PUBLISH_CMD",
}

rows = []
for op in plan.get("operations", []):
    op_id = op.get("id", "unknown")
    ready = bool(op.get("ready"))
    missing = op.get("missing") or []
    cmd = os.environ.get(adapter_env.get(op_id, ""), "")
    has_unresolved_placeholder = "REPLACE_ME_" in cmd

    row = {
        "id": op_id,
        "ready": ready,
        "mode": mode,
        "status": "planned",
        "missing": missing,
        "adapter_env": adapter_env.get(op_id),
        "adapter_configured": bool(cmd) and not has_unresolved_placeholder,
        "adapter_has_unresolved_placeholder": has_unresolved_placeholder,
        "publish_hint": op.get("publish_hint"),
    }

    if not ready:
        row["status"] = "deferred"
        rows.append(row)
        continue

    if mode == "dry-run":
        row["status"] = "planned"
        if has_unresolved_placeholder:
            row["warning"] = "adapter command contains unresolved REPLACE_ME_* placeholders"
        rows.append(row)
        continue

    if not cmd:
        row["status"] = "skipped"
        row["error"] = "adapter command not configured"
        rows.append(row)
        continue

    if has_unresolved_placeholder:
        row["status"] = "error"
        row["error"] = "adapter command contains unresolved REPLACE_ME_* placeholders"
        rows.append(row)
        continue

    proc = subprocess.run(["bash", "-lc", cmd], text=True, capture_output=True)
    log_line = f"[{op_id}] cmd={cmd} rc={proc.returncode}\n"
    run_log.parent.mkdir(parents=True, exist_ok=True)
    with run_log.open("a", encoding="utf-8") as f:
        f.write(log_line)
        if proc.stdout:
            f.write(proc.stdout + "\n")
        if proc.stderr:
            f.write(proc.stderr + "\n")

    if proc.returncode == 0:
        row["status"] = "published"
    else:
        row["status"] = "error"
        row["error"] = proc.stderr.strip() or proc.stdout.strip() or "adapter failed"
    rows.append(row)

ok = all(r["status"] in ("planned", "published") for r in rows)
partial = any(r["status"] in ("deferred", "skipped") for r in rows)

status = "ok" if ok and not partial else "partial"
if any(r["status"] == "error" for r in rows):
    status = "error"

report = {
    "schema": "nc.phase10.external.publish.report.v1",
    "version": "2.0.0",
    "state": "frozen",
    "generated_at": datetime.datetime.now(datetime.timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z"),
    "mode": mode,
    "status": status,
    "plan": plan,
    "operations": rows,
    "run_log": str(run_log),
}
print(json.dumps(report, indent=2, ensure_ascii=True))
PY
)"

mkdir -p "$(dirname "$REPORT_PATH")"
printf '%s\n' "$PY_STATUS" > "$REPORT_PATH"

FINAL_STATUS="$(python3 - <<'PY' "$REPORT_PATH"
import json,sys
p=json.loads(open(sys.argv[1],encoding='utf-8').read())
print(p.get('status','error'))
PY
)"

if [[ "$FINAL_STATUS" == "error" ]]; then
  cat "$REPORT_PATH"
  echo "ERROR: external publish report status=error" >&2
  exit 1
fi
if [[ "$FINAL_STATUS" == "partial" && $ALLOW_PARTIAL -eq 0 ]]; then
  cat "$REPORT_PATH"
  echo "ERROR: external publish report status=partial (use --allow-partial to accept deferred adapters)" >&2
  exit 1
fi

echo "OK: external publication plan/report -> $REPORT_PATH (status=$FINAL_STATUS)"
