#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

readonly P2_LOCKED_F10_P1_ARTIFACTS=(
  "docs/status/F10_P1_low_level_extension_migration_wave1_report.json"
  "docs/status/F10_P1_low_level_extension_migration_wave2_report.json"
  "docs/status/F10_P1_low_level_extension_migration_wave3_report.json"
  "docs/status/F10_P1_low_level_extension_migration_wave4_report.json"
  "docs/status/F10_P1_distribution_report.json"
  "docs/status/F10_P1_clean_install_report.json"
)

declare -A BEFORE_HASHES=()
for rel in "${P2_LOCKED_F10_P1_ARTIFACTS[@]}"; do
  if [[ ! -f "$rel" ]]; then
    echo "qa_phase10_p2_v2_0_0: missing locked F10-P1 artifact '$rel'" >&2
    exit 1
  fi
  BEFORE_HASHES["$rel"]="$(sha256sum "$rel" | awk '{print $1}')"
done

cargo test -p ncc-cli release_phase10_toolchain_model_gate_tests -- --nocapture
cargo test -p ncc-cli release_phase10_release_train_gate_tests -- --nocapture

tools/release/snapshot_phase10_toolchain_model_v2_0_0.sh

for rel in "${P2_LOCKED_F10_P1_ARTIFACTS[@]}"; do
  after="$(sha256sum "$rel" | awk '{print $1}')"
  if [[ "${BEFORE_HASHES[$rel]}" != "$after" ]]; then
    echo "qa_phase10_p2_v2_0_0: locked F10-P1 artifact changed during F10-P2 gate: $rel" >&2
    exit 1
  fi
done

echo "OK: qa_phase10_p2_v2_0_0 complete"
