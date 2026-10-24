#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

usage() {
  cat <<'USAGE'
Usage: publish_phase10_ide_marketplaces_v2_0_0.sh [options]

Options:
  --dist-dir <dir>          IDE distribution root (default: var/releases/v2_0_0/phase10_ide_dist)
  --mode <mode>             dry-run|execute (default: dry-run)
  --ops <csv>               all|jetbrains,vscode,ghidra (default: all)
  --report <file>           JSON report path (default: docs/status/F10_P20_ide_marketplace_publish_report.json)
  --allow-partial           Accept deferred/skipped operations
  --help                    Show help

Environment adapters (execute mode):
  NC_JETBRAINS_PUBLISH_CMD
  NC_VSCODE_PUBLISH_CMD
  NC_GHIDRA_PUBLISH_CMD
USAGE
}

DIST_DIR="var/releases/v2_0_0/phase10_ide_dist"
MODE="dry-run"
OPS="all"
REPORT_PATH="docs/status/F10_P20_ide_marketplace_publish_report.json"
ALLOW_PARTIAL=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dist-dir)
      shift
      [[ $# -gt 0 ]] || { echo "missing value for --dist-dir" >&2; exit 2; }
      DIST_DIR="$1"
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

case "$MODE" in
  dry-run|execute) ;;
  *) echo "invalid --mode '$MODE'" >&2; exit 2 ;;
esac

if [[ "$OPS" != "all" ]]; then
  IFS=',' read -r -a _ops <<< "$OPS"
  for op in "${_ops[@]}"; do
    case "$(echo "$op" | tr '[:upper:]' '[:lower:]' | xargs)" in
      jetbrains|vscode|ghidra) ;;
      *) echo "invalid --ops selector '$op'" >&2; exit 2 ;;
    esac
  done
fi

DIST_PATH="$ROOT_DIR/$DIST_DIR"
if [[ ! -d "$DIST_PATH" ]]; then
  echo "missing dist dir: $DIST_PATH" >&2
  exit 1
fi

TMP_DIR="$(mktemp -d)"
PLAN_JSON="$TMP_DIR/plan.json"
RUN_LOG="$TMP_DIR/run.log"

python3 - <<'PY' "$DIST_PATH" "$PLAN_JSON" "$OPS"
import json
import pathlib
import sys

dist = pathlib.Path(sys.argv[1])
out = pathlib.Path(sys.argv[2])
ops_selector = sys.argv[3].strip().lower()

jetbrains = next((p for p in (dist / "jetbrains").glob("*.zip") if p.is_file()), None)
vscode = next((p for p in (dist / "vscode").glob("*.vsix") if p.is_file()), None)
ghidra = dist / "ghidra" / "nc-ghidra-extension-v2_0_0.zip"

ops = [
    {
        "id": "jetbrains",
        "required": [str(jetbrains) if jetbrains else ""],
        "publish_hint": "gradle publishPlugin",
    },
    {
        "id": "vscode",
        "required": [str(vscode) if vscode else ""],
        "publish_hint": "vsce publish",
    },
    {
        "id": "ghidra",
        "required": [str(ghidra)],
        "publish_hint": "upload bundle to official-site/github-releases",
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
    "schema": "nc.phase10.ide.marketplace.publish.plan.v1",
    "version": "2.0.0",
    "state": "frozen",
    "distribution_root": str(dist),
    "operations": ops,
}
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(json.dumps(payload, indent=2, ensure_ascii=True) + "\n", encoding="utf-8")
print(out)
PY

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
    "jetbrains": "NC_JETBRAINS_PUBLISH_CMD",
    "vscode": "NC_VSCODE_PUBLISH_CMD",
    "ghidra": "NC_GHIDRA_PUBLISH_CMD",
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
    run_log.parent.mkdir(parents=True, exist_ok=True)
    with run_log.open("a", encoding="utf-8") as f:
        f.write(f"[{op_id}] cmd={cmd} rc={proc.returncode}\n")
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
    "schema": "nc.phase10.ide.marketplace.publish.report.v1",
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
  echo "ERROR: ide marketplace publish report status=error" >&2
  exit 1
fi

if [[ "$FINAL_STATUS" == "partial" && $ALLOW_PARTIAL -eq 0 ]]; then
  cat "$REPORT_PATH"
  echo "ERROR: ide marketplace publish report status=partial (use --allow-partial)" >&2
  exit 1
fi

echo "OK: ide marketplace publication report -> $REPORT_PATH (status=$FINAL_STATUS)"
