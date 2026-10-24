#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

usage() {
  cat <<'USAGE'
Usage: promote_release_train_v2_0_0.sh <nightly|beta|stable> [--candidate <id>] [--dry-run] [--out <path>]
USAGE
}

if [ "$#" -lt 1 ]; then
  usage
  exit 2
fi

channel="$1"
shift

candidate=""
dry_run=false
out_path=""

while [ "$#" -gt 0 ]; do
  case "$1" in
    --candidate)
      shift
      [ "$#" -gt 0 ] || { echo "missing value for --candidate" >&2; exit 2; }
      candidate="$1"
      ;;
    --dry-run)
      dry_run=true
      ;;
    --out)
      shift
      [ "$#" -gt 0 ] || { echo "missing value for --out" >&2; exit 2; }
      out_path="$1"
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

case "$channel" in
  nightly|beta|stable) ;;
  *)
    echo "invalid channel '$channel' (expected nightly|beta|stable)" >&2
    exit 2
    ;;
esac

state_file="var/release/release_train_state_v2_0_0.json"
mkdir -p "$(dirname "$state_file")" var/release/train/promotions
if [ ! -f "$state_file" ]; then
  cat > "$state_file" <<'JSON'
{
  "schema": "ncc.release.train.state.v1",
  "version": "2.0.0",
  "series": "2.x",
  "channels": {
    "nightly": {"candidate": null, "promoted_at": null},
    "beta": {"candidate": null, "promoted_at": null},
    "stable": {"candidate": null, "promoted_at": null}
  }
}
JSON
fi

if [ -z "$candidate" ]; then
  candidate="${RELEASE_CANDIDATE_ID:-${GITHUB_SHA:-}}"
fi

if [ "$channel" = "nightly" ] && [ -z "$candidate" ]; then
  echo "candidate is required for nightly promotion" >&2
  exit 2
fi

if [ -z "$out_path" ]; then
  ts="$(date -u +%Y%m%dT%H%M%SZ)"
  out_path="var/release/train/promotions/${ts}_${channel}_v2_0_0.json"
fi

required_gates=(
  release_phase1_language_base_gate_tests
  release_phase2_toolchain_model_gate_tests
  release_phase3_runtime_stdlib_gate_tests
  release_phase4_backend_sovereign_gate_tests
  release_phase5_toolchain_complete_gate_tests
  release_phase6_quality_extreme_gate_tests
  release_phase7_package_ecosystem_gate_tests
  release_phase8_dx_observability_gate_tests
  release_phase9_governance_release_gate_tests
  release_phase10_installer_gate_tests
  release_phase10_toolchain_model_gate_tests
  release_phase10_distribution_os_gate_tests
  release_phase10_disk_layout_gate_tests
  release_phase10_ncup_commands_gate_tests
  release_phase10_toolchain_resolution_gate_tests
  release_phase10_supply_chain_security_gate_tests
  release_phase10_manifest_format_gate_tests
  release_phase10_updates_gate_tests
  release_phase10_jetbrains_gate_tests
  release_phase10_vscode_gate_tests
  release_phase10_ghidra_gate_tests
  release_phase10_file_association_global_exec_gate_tests
  release_phase10_first_run_gate_tests
  release_phase10_cicd_compat_gate_tests
  release_phase10_telemetry_gate_tests
  release_phase10_installer_matrix_gate_tests
  release_phase10_support_policy_gate_tests
  release_phase10_documentation_gate_tests
  release_phase10_final_closure_gate_tests
  release_phase10_release_train_gate_tests
)

if [ "${NCC_SKIP_PROMOTION_GATES:-0}" != "1" ]; then
  for gate in "${required_gates[@]}"; do
    cargo test -p ncc-cli "$gate" -- --nocapture
  done
  cargo test -p ncc-cli release_f23_compatibility_gate_tests -- --nocapture
  cargo test -p ncc-cli release_f24_release_engineering_gate_tests -- --nocapture
fi

python3 - <<'PY' "$state_file" "$channel" "$candidate" "$dry_run" "$out_path"
import json,sys,datetime,os
state_file, channel, candidate, dry_run, out_path = sys.argv[1:6]
dry_run = dry_run.lower() == 'true'
with open(state_file,'r',encoding='utf-8') as f:
    state=json.load(f)
channels=state.setdefault('channels',{})
for name in ('nightly','beta','stable'):
    channels.setdefault(name,{"candidate":None,"promoted_at":None})

if channel in ('beta','stable') and not candidate:
    source = 'nightly' if channel=='beta' else 'beta'
    candidate = channels.get(source,{}).get('candidate')

if not candidate:
    raise SystemExit(f"cannot resolve candidate for {channel} promotion")

now=datetime.datetime.now(datetime.timezone.utc).replace(microsecond=0).isoformat().replace("+00:00","Z")
plan={
  "schema":"ncc.release.train.promotion.v1",
  "version":"2.0.0",
  "series":"2.x",
  "channel":channel,
  "candidate":candidate,
  "dry_run":dry_run,
  "promoted_at":now,
  "required_gates":[
    "release_phase1_language_base_gate_tests",
    "release_phase2_toolchain_model_gate_tests",
    "release_phase3_runtime_stdlib_gate_tests",
    "release_phase4_backend_sovereign_gate_tests",
    "release_phase5_toolchain_complete_gate_tests",
    "release_phase6_quality_extreme_gate_tests",
    "release_phase7_package_ecosystem_gate_tests",
    "release_phase8_dx_observability_gate_tests",
    "release_phase9_governance_release_gate_tests",
    "release_phase10_installer_gate_tests",
    "release_phase10_toolchain_model_gate_tests",
    "release_phase10_distribution_os_gate_tests",
    "release_phase10_disk_layout_gate_tests",
    "release_phase10_ncup_commands_gate_tests",
    "release_phase10_toolchain_resolution_gate_tests",
    "release_phase10_supply_chain_security_gate_tests",
    "release_phase10_manifest_format_gate_tests",
    "release_phase10_updates_gate_tests",
    "release_phase10_jetbrains_gate_tests",
    "release_phase10_vscode_gate_tests",
    "release_phase10_ghidra_gate_tests",
    "release_phase10_file_association_global_exec_gate_tests",
    "release_phase10_first_run_gate_tests",
    "release_phase10_cicd_compat_gate_tests",
    "release_phase10_telemetry_gate_tests",
    "release_phase10_installer_matrix_gate_tests",
    "release_phase10_support_policy_gate_tests",
    "release_phase10_documentation_gate_tests",
    "release_phase10_final_closure_gate_tests",
    "release_phase10_release_train_gate_tests",
    "release_f23_compatibility_gate_tests",
    "release_f24_release_engineering_gate_tests"
  ],
  "predictability_contract":"users know exactly what breaks, when and why"
}
os.makedirs(os.path.dirname(out_path),exist_ok=True)
with open(out_path,'w',encoding='utf-8') as f:
    json.dump(plan,f,indent=2,ensure_ascii=False)

if not dry_run:
    channels[channel] = {"candidate": candidate, "promoted_at": now}
    with open(state_file,'w',encoding='utf-8') as f:
        json.dump(state,f,indent=2,ensure_ascii=False)

print(f"OK: release-train v2 promotion plan {out_path}")
if dry_run:
    print("warning: dry-run active, state file not updated")
else:
    print(f"OK: release-train state updated for channel '{channel}'")
PY
