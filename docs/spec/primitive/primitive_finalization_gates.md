# nC primitive[@] — Finalization Gates (Closure Addendum)

Status: BASE ACTIVE
Scope: strict closure gates for primitive[@] acceptance

## 1. Purpose

This addendum closes the final operational gaps for primitive[@] with strict,
auditable gates:

1. exact official test matrix verification (one-by-one),
2. diagnostic code uniformity verification (`E_PRIMITIVE_*`),
3. strict no-new-debt gate (warning tokens, TODO/FIXME, stubs/placeholders),
4. final acceptance run (single uninterrupted run + single consolidated report),
5. industrial freestanding ELF cross-validation evidence.

## 2. Mandatory Gate Scripts

The following scripts are mandatory for primitive closure:

- `tools/scripts/primitive_verify_test_matrix.sh`
- `tools/scripts/primitive_gate_diagnostics_uniformity.sh`
- `tools/scripts/primitive_gate_strict_no_new_debt.sh`
- `tools/scripts/primitive_final_acceptance_report.sh`

## 3. Baseical Matrix Source

Baseical expected test names are declared in:

- `tools/gates/primitive_test_matrix_expected.txt`
- scenario-equivalence mapping:
  `tools/gates/primitive_test_matrix_mapping.regex`

Rule:

- Every official scenario name MUST be discoverable in `cargo test -- --list`.
- If exact names differ, an approved scenario-equivalence regex MUST match.
- Missing official scenarios are closure blockers.

## 4. Diagnostic Uniformity Rule

Primitive diagnostic paths MUST emit coded primitive diagnostics (`E_PRIMITIVE_*`).

Rule:

- Baseical code set is declared in:
  `tools/gates/primitive_expected_codes.txt`.
- Every code in that set MUST be present in sema/cli enforcement surfaces.

## 5. Strict Debt Rule (New Debt Only)

`primitive_gate_strict_no_new_debt.sh` enforces a strict no-new-debt policy.

Rule:

- New warning-token matches (`TODO`, `FIXME`, `stub`, `placeholder`, etc.) in
  primitive closure scope fail the gate.
- Historical or non-closure artifacts may be allowlisted only in
  `tools/gates/primitive_debt_allowlist.regex`.

## 6. Sensory Lowering Policy

`print.sensory` is semantically gated and policy-valid only with:

- primitive mode,
- sensory bridge,
- expert profile,
- authority constraints.

Material lowering rule:

- If sensory backend is unavailable for the selected target, compiler MUST emit:
  `E_PRIMITIVE_SENSORY_LOWERING_UNAVAILABLE_001`.

This is a official explicit failure mode, not silent fallback.

## 7. Industrial ELF Cross-Validation

The final acceptance runner includes:

- freestanding ELF build for `x86_64-unknown-none`,
- `readelf -h` / `readelf -l` validation when available,
- QEMU serial smoke validation through
  `tools/scripts/run_primitive_qemu_smoke.sh`.

Rule:

- Primitive closure requires real artifact evidence, not metadata-only claims.

## 8. Single Final Acceptance Report

`tools/scripts/primitive_final_acceptance_report.sh` executes the full closure
sequence and emits:

- `FINAL_ACCEPTANCE_REPORT.md`
- `FINAL_ACCEPTANCE_REPORT.json`
- per-step logs and status

Rule:

- Closure decisions MUST use this consolidated report as source of truth.
