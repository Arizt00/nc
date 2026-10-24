#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "[F9] governance + release gate"
cargo test -p ncc-cli release_phase9_governance_release_gate_tests -- --nocapture

echo "[F9] regression gates"
cargo test -p ncc-cli release_phase8_dx_observability_gate_tests -- --nocapture
cargo test -p ncc-cli release_phase7_package_ecosystem_gate_tests -- --nocapture

echo "[F9] release-train promotion smoke (dry-run)"
NCC_SKIP_PROMOTION_GATES=1 tools/release/promote_release_train_v1_0_1.sh nightly --candidate local-smoke --dry-run

echo "[F9] artifact integrity smoke"
tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT
mkdir -p "$tmp_dir/dist"
printf 'ncc-phase9-smoke\n' > "$tmp_dir/dist/smoke.bin"
export NCC_RELEASE_SIGNING_KEY="phase9-smoke-key"
tools/release/sign_artifacts_v1_0_1.sh "$tmp_dir/dist" "$tmp_dir/integrity"
tools/release/verify_artifacts_v1_0_1.sh "$tmp_dir/dist" "$tmp_dir/integrity"

echo "[F9] runbook SLA smoke"
tools/release/runbook_lts_enterprise_sla_v1_0_1.sh

echo "[F9] governance + release suite OK"
