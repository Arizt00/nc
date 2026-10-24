# nC - B1 Symbol Resolution Policy

Status: Active Base (B1.6)
Version: v2.0.0-B1

## 1. Scope

This policy closes unresolved symbol handling across backend lanes.

Typed implementation:

- `BackendObjectIR::unresolved_symbols()`
- `BackendUnresolvedSymbolPolicyIR`
- `BackendTargetSpecIR::linking_contract()`
- enforcement in `emit_object_elf64(...)`

## 2. Lane Policy

### Hosted lane

- unresolved symbols are allowed at backend object/ELF stage;
- they are treated as imported symbols for host linker resolution.

Policy enum:

- `AllowImportedSymbols`

### Portable / Freestanding / Sovereign lanes

- unresolved symbols are rejected at backend stage;
- no linker spawn is allowed to hide unresolved backend contracts.

Policy enum:

- `RejectUnresolvedAtBackendStage`

## 3. Baseical Diagnostic

When policy is reject and unresolved symbols exist:

- `symbol unresolved at backend stage: <sym0>, <sym1>, ...`

## 4. Integration with build-exe

`build_executable_from_object_with_linker(...)` now inherits this behavior through `emit_object_elf64(...)`.

Result:

- in reject lanes, unresolved symbol failure occurs before external linker execution.

## 5. Conformance Tests

IR:

- hosted lane unresolved import allowed in ELF emission;
- freestanding lane unresolved symbol rejected in ELF emission.

CLI:

- build-exe rejects unresolved freestanding object before linker spawn.
