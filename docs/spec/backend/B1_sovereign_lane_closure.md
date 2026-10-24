# nC - B1 Sovereign Lane Native Closure

Status: Active Base (B1.9)
Version: v2.0.0-B1

## 1. Scope

This document freezes native sovereign-lane emission for B1.9.

It closes:

- official sovereign backend profile selection;
- typed artifact contract for sovereign-native output;
- deterministic sovereign image emission path in `ncc-ir`;
- CLI commands bound to typed lane validation.

Follow-up closure:

- B1.10 freezes sovereign internal linker execution for `build-sovereign` in:
  - `docs/spec/backend/B1_sovereign_internal_linker_closure.md`

## 2. Baseical Sovereign Profile

Baseical profile:

- `nC.sovereign`

Resolved backend target contract:

- triple: `x86_64-unknown-sovereign-nc`
- lane: `sovereign`
- runtime model: `Sovereign`
- object format: `freestanding-image`
- linker model: `sovereign-internal`
- tier: `Tier3`

## 3. Typed Artifact Contract

Sovereign lane supported artifacts:

- `emit-backend`
- `emit-object`
- `emit-sovereign`
- `build-sovereign`

Sovereign lane rejected artifacts include hosted/portable artifacts outside its contract, including:

- `emit-elf`
- `build-exe`
- `emit-wasm`

## 4. Native Sovereign Emission

Implemented in `ncc-ir`:

- `emit_image_sovereign(&BackendObjectIR) -> Result<Vec<u8>, String>`

Contract:

- requires lane `sovereign`;
- requires object format `freestanding-image`;
- enforces unresolved symbol policy at backend stage;
- emits deterministic native image bytes with official header magic:
  - `NCSOVIMG`

Baseical diagnostics:

- `sovereign image emission requires lane 'sovereign' with object format 'freestanding-image'`
- `symbol unresolved at backend stage: <...>`

## 5. CLI Integration

Implemented in `ncc-cli`:

- `--cc nC.sovereign`
- `emit-sovereign <file.n> <out.ncsov> [--cc nC.sovereign]`
- `build-sovereign <file.n> <out.ncsov> [--cc nC.sovereign]`

CLI artifact validation is still routed through:

- `validate_backend_artifact_profile(...)`

This keeps lane enforcement typed and centralized.

## 6. Conformance Evidence

Core tests:

- `abi_ir_tests::backend_target_spec_sovereign_is_tier3_sovereign_with_freestanding_image`
- `abi_ir_tests::backend_linking_contract_sovereign_lane_supports_native_image_artifacts`
- `backend_emit_ir_tests::emit_image_sovereign_materializes_native_image_header_and_counts`
- `backend_emit_ir_tests::emit_image_sovereign_rejects_unresolved_symbols_at_backend_stage`
- `backend_emit_cli_tests::parse_backend_profile_accepts_sovereign`
- `backend_emit_cli_tests::backend_artifact_profile_rejects_emit_elf_for_sovereign_lane`
- `backend_emit_cli_tests::emit_sovereign_with_sovereign_profile_materializes_native_image`

Validation runs:

- `cargo check -p ncc-ir -p ncc-cli`
- `cargo test -p ncc-ir --lib -- --nocapture`
- `cargo test -p ncc-cli --lib -- --nocapture`
