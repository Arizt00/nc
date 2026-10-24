# nC - B1 Portable Lane Native Closure

Status: Active Base (B1.8)
Version: v2.0.0-B1

## 1. Scope

This document freezes native portable-lane execution and emission for B1.8.

## 2. Baseical Portable Profile

Profile:

- `nC.portable`

Target model:

- triple: `wasm32-wasi`
- family: `Portable`
- ISA: `wasm32`
- ABI: `WASI`
- format: `wasm-module`
- runtime: `Portable`
- tier: `Tier1`

## 3. Lane Artifacts

Portable lane supports:

- `emit-backend`
- `emit-object`
- `emit-wasm`
- `build-wasm`

Portable lane rejects:

- `emit-elf`
- `emit-coff`
- `emit-macho`
- `build-exe`

through typed linking-contract diagnostics.

## 4. Native Wasm Emission

Implemented in `ncc-ir`:

- `emit_module_wasm32(&BackendModuleIR) -> Result<Vec<u8>, String>`

Contract:

- deterministic module bytes;
- valid wasm magic/version;
- `_start` export materialized;
- exported function symbols retained for tooling visibility.

## 5. CLI Surface

Implemented in `ncc-cli`:

- `ncc emit-wasm <file.n> <out.wasm> --cc nC.portable`
- `ncc build-wasm <file.n> <out.wasm> --cc nC.portable`

## 6. Conformance Locks

Covered by tests:

- `abi_ir_tests::backend_target_spec_portable_is_tier1_portable_with_wasm_module`
- `abi_ir_tests::backend_linking_contract_portable_lane_supports_wasm_artifacts`
- `abi_ir_tests::backend_link_execution_contract_portable_is_internal_module_driver`
- `backend_emit_ir_tests::emit_module_wasm32_portable_lane_materializes_valid_module_header_and_start_export`
- `backend_emit_cli_tests::parse_backend_profile_accepts_portable`
- `backend_emit_cli_tests::backend_artifact_profile_rejects_emit_elf_for_portable_lane`
- `backend_emit_cli_tests::emit_wasm_with_portable_profile_materializes_wasm_module`
