# Runtime Type Contracts

Runtime enforces executable parts of `type{}` contracts through existing value/runtime checks:
- constructure/destructure runtime paths
- copy/clone/move behavior
- bounds and invariant failure surfaced as typed diagnostics
- binary/unbinary stability checks for memory-sensitive operations

Current runtime strategy remains explicit:
- no silent fallback
- no placeholder value substitution
- no implicit mode crossing

## Runtime Materialization (v2.1.0)

During execution, `ncc-cli` now builds a runtime type-contract catalog from AST `TypeContracts` sections and applies it in real assignment/initialization paths.

Validated clauses at runtime:
- `fields` (required field presence + field type checks)
- `bounds` (`range`, `len`, `nonEmpty`, `nonnull`)
- `invariant` (integer comparator subset: `==`, `!=`, `>`, `>=`, `<`, `<=`)
- `nullability` (`nullable` vs non-null contracts)
- `ownership` (`owned|borrowed|shared|moved` contract vocabulary)
- `drop` presence tracking on symbol replacement paths

Primary runtime diagnostics now emitted from contract validation:
- `E_TYPE_FIELD_MISSING_001`
- `E_TYPE_FIELD_TYPE_001`
- `E_TYPE_FIELD_UNKNOWN_001`
- `E_TYPE_BOUNDS_RANGE_001`
- `E_TYPE_BOUNDS_LEN_001`
- `E_TYPE_FIELD_BOUNDS_001`
- `E_TYPE_NULLABILITY_001`
- `E_TYPE_OWNERSHIP_001`
- `E_TYPE_INVARIANT_RUNTIME_FAIL_001`
- `E_TYPE_STRICTLY_ACCESS_001`

## Runtime Alias Surface (v2.1.0)

Runtime now exposes explicit alias calls backed by `type{}` clauses:

- `compare.<name>(lhs, rhs)` / `type.compare.<name>(lhs, rhs)`
- `hash.<name>(value)` / `type.hash.<name>(value)`
- `debug.<name>(value)` / `type.debug.<name>(value)`
- `stable.<name>(value)` / `type.stable.<name>(value)`

These paths are contract-gated:

- compare requires declared `compare:` clause on the resolved type contract
- hash requires declared `hash:` clause (and `stable:` when using `hash.stable`)
- debug requires declared `debug:` clause
- stable requires declared `stable:` clause

No implicit fallback is performed when clause declarations are missing.

## Scope

This runtime pass focuses on executable contract checks in high/intermediate profiles.
Deep IR-native lowering of every contract clause remains a separate compiler/nIR closure track.
