#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

cargo test -p ncc-cli release_phase10_distribution_os_gate_tests -- --nocapture

# Host-only execution validates full Linux path in this environment and emits official reports.
P3_BUILD_LOG="$(mktemp -t nc_phase10_p3_build_XXXX.log)"
trap 'rm -f "$P3_BUILD_LOG"' EXIT
tools/release/build_phase10_distribution_os_v2_0_0.sh --host-only 2>&1 | tee "$P3_BUILD_LOG"

if grep -E -n "rpmdb\\.sqlite|cannot open Packages database|bogus date in %changelog" "$P3_BUILD_LOG" >/dev/null; then
  echo "qa_phase10_p3_v2_0_0: detected forbidden rpm warning signature in distribution pipeline output" >&2
  grep -E -n "rpmdb\\.sqlite|cannot open Packages database|bogus date in %changelog" "$P3_BUILD_LOG" >&2 || true
  exit 1
fi

"${NCC_RELEASE_VERIFY:-ncc-release}" release-layout \
  --staging var/releases/v2_0_0/phase10_p3_dist/staging \
  --report docs/status/F10_P3_distribution_os_report.json

echo "OK: qa_phase10_p3_v2_0_0 complete"
