#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_PATH="${1:-$ROOT_DIR/docs/status/F10_P2_toolchain_model_report.json}"
TMP_PHASE2="$(mktemp)"
TMP_PHASE10="$(mktemp)"
trap 'rm -f "$TMP_PHASE2" "$TMP_PHASE10"' EXIT

cd "$ROOT_DIR"

cargo run -q -p ncc-cli --bin ncc -- toolchain-model --json > "$TMP_PHASE2"
cargo run -q -p ncc-cli --bin ncc -- phase10-toolchain-model --json > "$TMP_PHASE10"

python3 - "$TMP_PHASE2" "$TMP_PHASE10" "$OUT_PATH" <<'PY'
import json
import sys
from datetime import datetime, timezone

phase2_path, phase10_path, out_path = sys.argv[1:4]
with open(phase2_path, "r", encoding="utf-8") as f:
    phase2 = json.load(f)
with open(phase10_path, "r", encoding="utf-8") as f:
    phase10 = json.load(f)

expected_channels = ["stable", "beta", "nightly"]
expected_components = ["compiler", "runtime", "stdlib", "fmt", "lint", "lsp", "docs", "sources"]
expected_profiles = {
    "minimal": ["compiler", "runtime", "stdlib"],
    "default": ["compiler", "runtime", "stdlib", "fmt", "lint", "lsp", "docs"],
    "complete": ["compiler", "runtime", "stdlib", "fmt", "lint", "lsp", "docs", "sources"],
}

phase2_ids = sorted((row.get("id") for row in phase2.get("toolchains", []) if isinstance(row, dict) and isinstance(row.get("id"), str)))
phase10_ids = sorted((row.get("id") for row in phase10.get("toolchains", []) if isinstance(row, dict) and isinstance(row.get("id"), str)))

checks = {
    "phase2_version_frozen": phase2.get("version") == "1.0.1" and phase2.get("state") == "frozen",
    "phase10_schema_version_frozen": phase10.get("schema") == "nc.phase10.toolchain.model.v1" and phase10.get("version") == "2.0.0" and phase10.get("state") == "frozen",
    "channels_match": phase2.get("channels") == phase10.get("channels") == expected_channels,
    "components_match": phase2.get("components") == phase10.get("components") == expected_components,
    "profiles_match": phase10.get("install_profiles") == expected_profiles,
    "toolchain_ids_match": phase2_ids == phase10_ids,
    "required_gate_release_train_bound": "release_phase10_release_train_gate_tests" in (phase10.get("required_gates") or []),
    "owned_scope_declared": "tools/release/qa_phase10_p2_v2_0_0.sh" in (phase10.get("owned_scope_artifacts") or []),
    "locked_scope_declared": "docs/status/F10_P1_distribution_report.json" in (phase10.get("locked_external_artifacts") or []),
    "locked_scope_clean_install_declared": "docs/status/F10_P1_clean_install_report.json" in (phase10.get("locked_external_artifacts") or []),
    "has_stable_linux": "stable-x86_64-unknown-linux-gnu" in phase10_ids,
    "has_stable_wasi": "stable-wasm32-wasi" in phase10_ids,
    "has_beta_linux_aarch64": "beta-aarch64-unknown-linux-gnu" in phase10_ids,
    "has_nightly_sovereign": "nightly-x86_64-unknown-sovereign-nc" in phase10_ids,
}

report = {
    "schema": "nc.phase10.toolchain.report.v1",
    "version": "2.0.0",
    "state": "frozen",
    "checked_at_utc": datetime.now(timezone.utc).isoformat(),
    "phase2": {
        "version": phase2.get("version"),
        "state": phase2.get("state"),
        "toolchain_count": len(phase2_ids),
    },
    "phase10": {
        "schema": phase10.get("schema"),
        "version": phase10.get("version"),
        "state": phase10.get("state"),
        "toolchain_count": len(phase10_ids),
    },
    "checks": checks,
}

with open(out_path, "w", encoding="utf-8") as f:
    json.dump(report, f, indent=2)

if not all(checks.values()):
    failed = [k for k, v in checks.items() if not v]
    raise SystemExit("phase10 toolchain snapshot checks failed: " + ", ".join(failed))
PY

echo "OK: phase10 toolchain snapshot -> $OUT_PATH"
