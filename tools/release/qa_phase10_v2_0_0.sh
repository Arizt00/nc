#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "[F10-P1] installer + ecosystem gate"
cargo test -p ncc-cli release_phase10_installer_gate_tests -- --nocapture

echo "[F10-P2] toolchain model gate"
cargo test -p ncc-cli release_phase10_toolchain_model_gate_tests -- --nocapture
cargo test -p ncc-cli release_phase10_release_train_gate_tests -- --nocapture
tools/release/snapshot_phase10_toolchain_model_v2_0_0.sh

echo "[F10-P3] distribution by OS gate"
tools/release/qa_phase10_p3_v2_0_0.sh

echo "[F10-P4] official disk layout gate"
tools/release/qa_phase10_p4_v2_0_0.sh

echo "[F10-P5] ncup mandatory commands gate"
tools/release/qa_phase10_p5_v2_0_0.sh

echo "[F10-P6] deterministic toolchain resolution gate"
tools/release/qa_phase10_p6_v2_0_0.sh

echo "[F10-P7] supply chain security gate"
tools/release/qa_phase10_p7_v2_0_0.sh

echo "[F10-P8] manifest format gate"
tools/release/qa_phase10_p8_v2_0_0.sh

echo "[F10-P9] updates transactional+rollback gate"
tools/release/qa_phase10_p9_v2_0_0.sh

echo "[F10-P10] jetbrains integration gate"
tools/release/qa_phase10_p10_v2_0_0.sh

echo "[F10-P11] vscode integration gate"
tools/release/qa_phase10_p11_v2_0_0.sh

echo "[F10-P20] ghidra integration gate"
tools/release/qa_phase10_p20_v2_0_0.sh

echo "[F10-IDE] fullstack ide gate"
tools/release/qa_phase10_ide_fullstack_v2_0_0.sh

echo "[F10-P12] file association + global execution gate"
tools/release/qa_phase10_p12_v2_0_0.sh

echo "[F10-P13] first-run experience gate"
tools/release/qa_phase10_p13_v2_0_0.sh

echo "[F10-P14] ci/cd compatibility gate"
tools/release/qa_phase10_p14_v2_0_0.sh

echo "[F10-P15] telemetry gate"
tools/release/qa_phase10_p15_v2_0_0.sh

echo "[F10-P16] installer matrix gate"
tools/release/qa_phase10_p16_v2_0_0.sh

echo "[F10-P17] support policy gate"
tools/release/qa_phase10_p17_v2_0_0.sh

echo "[F10-P18] mandatory documentation gate"
tools/release/qa_phase10_p18_v2_0_0.sh

echo "[F10-P19] final closure gate"
tools/release/qa_phase10_p19_v2_0_0.sh

echo "[F10-P1] low-level extension migration wave1"
tools/release/migrate_low_level_extensions_v2_0_0.sh

echo "[F10-P1] distribution host bundle + integrity"
NCC_RELEASE_SIGNING_KEY="${NCC_RELEASE_SIGNING_KEY:-phase10-local-signing-key-v2}" \
  tools/release/build_phase10_ncup_distribution_v2_0_0.sh --host-only

echo "[F10-P1] clean install verification"
NCC_RELEASE_SIGNING_KEY="${NCC_RELEASE_SIGNING_KEY:-phase10-local-signing-key-v2}" \
  tools/release/verify_phase10_clean_install_v2_0_0.sh --key-env NCC_RELEASE_SIGNING_KEY

echo "[F10-P1] nclsp policy list"
cargo run -p ncc-cli -- nclsp policy --list

echo "[F10-P1] ncup model list"
cargo run -p ncc-cli -- ncup model --list

echo "[F10-P1] low-level lsp smoke"
if command -v go >/dev/null 2>&1; then
  go test tools/nc-sys-lsp/main.go tools/nc-sys-lsp/main_test.go
  go run tools/nc-sys-lsp/main.go info >/dev/null
  go run tools/nc-sys-lsp/main.go doctor >/dev/null
  go run tools/nc-sys-lsp/main.go analyze examples/root_at/root_at_minimal_ok.n --profile novice --migration-stage warning-first --json >/dev/null || true
  go run tools/nc-sys-lsp/main.go analyze examples/root_at/root_at_minimal_ok.nb --profile expert --migration-stage strict --json >/dev/null
  printf '%s\n' '{"id":"cap-1","op":"capabilities"}' | go run tools/nc-sys-lsp/main.go serve >/dev/null
  "${NCC_NATIVE_TOOLS:-ncc-tools-v43}" editor-lsp-smoke --root "$ROOT_DIR" \
    --out "$ROOT_DIR/target/tmp/phase10-lsp-native" >/dev/null
else
  echo "[F10-P1] go not found, skipping nc-sys-lsp runtime smoke"
fi

echo "[F10-OPS] external operations gate"
tools/release/qa_phase10_external_ops_v2_0_0.sh

echo "[PRIMITIVE] final material release evidence gate"
tools/release/qa_primitive_release_evidence_v2_0_0.sh

echo "[F10-P1] suite OK"
