# nC - B1 Linking Model Contract

Status: Active Base (B1.5 + B1.6 + B1.7 + B1.8 + B1.9 + B1.10 + B1.12)
Version: v2.0.0-B1

## 1. Scope

This document freezes linking contracts by backend lane and binds them to CLI artifact validation.

Typed contracts in `ncc-ir`:

- `BackendLinkArtifactIR`
- `BackendUnresolvedSymbolPolicyIR`
- `BackendLinkingContractIR`
- `BackendTargetSpecIR::linking_contract()`
- `BackendLoweringProfileIR::validate_linking_artifact(...)`

## 2. Lane Contracts

### Hosted lane

Supported artifacts:

- `emit-backend`
- `emit-object`
- `emit-elf`
- `emit-coff`
- `emit-macho`
- `build-exe`

Unresolved symbols:

- policy: `AllowImportedSymbols`

### Portable lane

Supported artifacts:

- `emit-backend`
- `emit-object`
- `emit-wasm`
- `build-wasm`

Unresolved symbols:

- policy: `RejectUnresolvedAtBackendStage`

### Freestanding lane

Supported artifacts:

- `emit-backend`
- `emit-object`
- `emit-elf`

Unresolved symbols:

- policy: `RejectUnresolvedAtBackendStage`

### Sovereign lane

Supported artifacts:

- `emit-backend`
- `emit-object`
- `emit-sovereign`
- `build-sovereign`

Unresolved symbols:

- policy: `RejectUnresolvedAtBackendStage`

## 3. Baseical Diagnostics Connected to CLI

`validate_backend_artifact_profile(...)` in `ncc-cli` now delegates to the typed contract.

Baseical diagnostics now surfaced through this path include:

- `object format unsupported for selected linking model: artifact '<...>' is unavailable in lane '<...>'`
- `backend artifact '<cmd>' is incompatible with calling convention nC.win64; use --cc nC.sysv64 or --cc nC.aapcs64 (or emit-coff for Win64 object output)`
- `backend artifact 'emit-macho' is incompatible with calling convention nC.win64; use --cc nC.sysv64 or --cc nC.aapcs64`

## 4. Active-Line Note

The lane-aware contract is frozen at B1.5.
Lane-level artifact compatibility is governed by typed backend policy instead of ad-hoc CLI branching.

## 5. Unresolved Symbol Policy (B1.6)

`emit_object_elf64(...)` enforces `BackendUnresolvedSymbolPolicyIR` from lane contract:

- hosted lanes may keep unresolved imported symbols;
- non-hosted lanes reject unresolved symbols at backend stage.

Baseical diagnostic:

- `symbol unresolved at backend stage: <sym...>`

Reference:

- `docs/spec/backend/B1_symbol_resolution_policy.md`

## 6. Link Execution Driver Contract (B1.7)

`BackendTargetSpecIR::link_execution_contract()` now freezes lane-level linker execution policy:

- hosted: `external-host-linker` (default linker: `cc`);
- portable: `portable-internal`;
- freestanding: `freestanding-internal`;
- sovereign: `sovereign-internal`.

`build-exe` in CLI now validates this contract:

- non-hosted internal lanes are rejected for external `build-exe` invocation with official diagnostic:
  - `linker model unsupported for selected backend lane: ...`

This keeps lane identity explicit and prevents accidental host-linker execution in non-hosted lanes.

## 7. Portable Lane Native Emission (B1.8)

B1.8 binds portable lane to native wasm artifacts:

- `emit-wasm`
- `build-wasm`

Backend emission path:

- `ncc_ir::emit_module_wasm32(&BackendModuleIR)` -> deterministic valid wasm module bytes.

CLI now enforces artifact validity:

- portable lane rejects `emit-elf` with typed lane diagnostic;
- portable lane accepts `emit-wasm` and `build-wasm`.

## 8. Sovereign Lane Native Emission (B1.9)

B1.9 binds sovereign lane to native image artifacts:

- `emit-sovereign`
- `build-sovereign`

Backend emission path:

- `ncc_ir::emit_image_sovereign(&BackendObjectIR)` -> deterministic sovereign image bytes.

CLI now enforces artifact validity:

- sovereign lane rejects `emit-elf` with typed lane diagnostic;
- sovereign lane accepts `emit-sovereign` and `build-sovereign`.

## 9. Sovereign Internal Linker Closure (B1.10)

B1.10 closes sovereign internal link execution for `build-sovereign`:

- `ncc_ir::link_image_sovereign(&BackendObjectIR, entry_symbol_override)`
- linked sovereign image header magic: `NCSOVLNK`
- deterministic section placement from official image base (`0x0010_0000`) with fixed alignment (`16`)
- relocation patching for:
  - `CallNear32`
  - `JumpNear32`
  - `Abs64`

Entry selection contract:

- explicit `--entry <symbol>` override when provided;
- otherwise default resolver policy:
  - `root.main` if present;
  - first global symbol ending in `.main`;
  - first global symbol;
  - first symbol in object;
  - hard error when object has no symbols.

CLI binding:

- `build-sovereign` now executes sovereign internal linking (not only native-image emission);
- `build-sovereign` accepts:
  - `--cc nC.sovereign` (optional, defaults to sovereign profile)
  - `--entry <symbol>` (optional explicit entry override)

Baseical diagnostics:

- `sovereign link execution requires lane 'sovereign' with object format 'freestanding-image'`
- `symbol unresolved at backend stage: <...>`
- `sovereign linker requires materialized native bytes for section '<...>' with relocations`
- `sovereign entry symbol '<...>' not found in linked object symbol table`

## 10. Native COFF/Mach-O Serializer Closure (B1.12)

B1.12 closes object serializer depth for hosted cross-target artifacts:

- `emit-coff` is backed by native `ncc-ir::emit_object_coff(...)`;
- `emit-macho` is backed by native `ncc-ir::emit_object_macho64(...)`.

Lane policy remains explicit:

- portable lane rejects hosted object artifact serializers;
- sovereign lane rejects hosted object artifact serializers;
- hosted lane accepts COFF/Mach-O serializers with typed backend object input.

Reference:

- `docs/spec/backend/B1_cross_target_object_serializers.md`
