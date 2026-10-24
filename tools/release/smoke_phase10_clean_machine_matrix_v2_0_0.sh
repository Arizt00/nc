#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

usage() {
  cat <<'USAGE'
Usage: smoke_phase10_clean_machine_matrix_v2_0_0.sh [options]

Run clean-machine smoke for ncup/ncc on current runner OS.

Options:
  --home <dir>          Explicit NCUP home (default: temp dir)
  --work <dir>          Explicit workspace for examples (default: temp dir)
  --report <file>       JSON report path (default: docs/status/F10_external_clean_machine_report.json)
  --skip-build          Skip cargo build step
  --help                Show help
USAGE
}

REPORT_PATH="docs/status/F10_external_clean_machine_report.json"
EXPLICIT_HOME=""
EXPLICIT_WORK=""
SKIP_BUILD=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --home)
      shift
      [[ $# -gt 0 ]] || { echo "missing value for --home" >&2; exit 2; }
      EXPLICIT_HOME="$1"
      ;;
    --work)
      shift
      [[ $# -gt 0 ]] || { echo "missing value for --work" >&2; exit 2; }
      EXPLICIT_WORK="$1"
      ;;
    --report)
      shift
      [[ $# -gt 0 ]] || { echo "missing value for --report" >&2; exit 2; }
      REPORT_PATH="$1"
      ;;
    --skip-build)
      SKIP_BUILD=1
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

if [[ $SKIP_BUILD -eq 0 ]]; then
  cargo build -p ncc-cli --bin ncc
fi

NCC_BIN="$ROOT_DIR/target/debug/ncc"
if [[ ! -x "$NCC_BIN" ]]; then
  echo "ncc binary not found: $NCC_BIN" >&2
  exit 1
fi

if [[ -n "$EXPLICIT_HOME" ]]; then
  HOME_DIR="$EXPLICIT_HOME"
else
  HOME_DIR="$(mktemp -d)"
fi
if [[ -n "$EXPLICIT_WORK" ]]; then
  WORK_DIR="$EXPLICIT_WORK"
else
  WORK_DIR="$(mktemp -d)"
fi
mkdir -p "$HOME_DIR" "$WORK_DIR"

HOST_TARGET="$(rustc -vV | awk '/^host:/ {print $2}')"

cat > "$WORK_DIR/hello.n" <<'NC'
helloFirstRun {
    to main {
        println("hello from clean machine");
        return 0;
    }

    fa:
        return true;
}
NC

cat > "$WORK_DIR/root_min.nb" <<'NC'
rootSurface[@] {
    stc:
        ok: bool;

    to main {
        @ok >> bool : true;
        print.sta(@ok);
    }

    fa:
        return true;
}
NC

commands_json="[]"
run_cmd() {
  local -a cmd=("$@")
  local out err rc
  out="$("${cmd[@]}" 2> >(cat >&2))" || rc=$?
  rc=${rc:-0}
  err=""
  commands_json="$(python3 - <<'PY' "$commands_json" "${cmd[*]}" "$rc" "$out" "$err"
import json,sys
arr=json.loads(sys.argv[1])
arr.append({"cmd":sys.argv[2],"code":int(sys.argv[3]),"stdout":sys.argv[4],"stderr":sys.argv[5],"ok":int(sys.argv[3])==0})
print(json.dumps(arr,ensure_ascii=True))
PY
)"
  return "$rc"
}

run_cmd "$NCC_BIN" ncup init --home "$HOME_DIR" --profile default --channel stable --target "$HOST_TARGET" --yes --force --json
run_cmd "$NCC_BIN" ncup doctor --home "$HOME_DIR"
run_cmd "$NCC_BIN" ncup which ncc --home "$HOME_DIR" --json
run_cmd "$NCC_BIN" run "$WORK_DIR/hello.n"
run_cmd "$NCC_BIN" nclsp validate "$WORK_DIR/root_min.nb" --profile expert --migration-stage strict --json

python3 - <<'PY' "$commands_json" "$REPORT_PATH" "$HOME_DIR" "$WORK_DIR" "$HOST_TARGET"
import datetime
import json
import pathlib
import sys

commands = json.loads(sys.argv[1])
report_path = pathlib.Path(sys.argv[2])
home_dir = pathlib.Path(sys.argv[3])
work_dir = pathlib.Path(sys.argv[4])
host_target = sys.argv[5]

ok = all(cmd.get("ok") for cmd in commands)

payload = {
    "schema": "nc.phase10.clean.machine.report.v1",
    "version": "2.0.0",
    "state": "frozen",
    "generated_at": datetime.datetime.now(datetime.timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z"),
    "status": "ok" if ok else "error",
    "host_target": host_target,
    "home": str(home_dir),
    "workspace": str(work_dir),
    "commands": commands,
}
report_path.parent.mkdir(parents=True, exist_ok=True)
report_path.write_text(json.dumps(payload, indent=2, ensure_ascii=True) + "\n", encoding="utf-8")
print(f"OK: clean-machine smoke report {report_path}")
if not ok:
    raise SystemExit(1)
PY
