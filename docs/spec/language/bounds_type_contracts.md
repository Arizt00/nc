# Bounds + Type{} Contract Surface (v2.1.0)

## Goal
Expose bounds and `type` contracts as explicit compiler/runtime artifacts for arrays, lists, indexing and memory operations.

## Implemented release model
This release now ships a dedicated `type:` section DSL in parser/AST/sema for normal/runtime/compiler high/intermediate containers.

Surface:

- parser: `type:` section with named contracts and clause blocks,
- AST: `Section::TypeContracts(Vec<TypeContractDecl>)` and `TypeContractSection`,
- sema: contract validation (`E_TYPE_*`) over fields/layout/invariant/ffi dependencies,
- matrix tests: official pass/fail names for type contracts, arrays/lists/index/memory.

Important boundary:

- `public/private/internal/strictly/layout/bounds/invariant/constructure/destructure/copy/clone/move/ffi`
  inside `type:` are **internal contract clauses**, not global container sections.
- these clauses do not require `[@primitive]`.

## `type:` DSL clauses supported
Inside each `type:` contract block:

- field clauses by visibility:
  - `public:`
  - `private:`
  - `internal:`
  - `strictly:`
- behavioral clauses:
  - `layout:`
  - `invariant:`
  - `bounds:`
  - `constructure:`
  - `destructure:`
  - `copy:`
  - `clone:`
  - `move:`
  - `ffi:`
  - `ownership:`
  - `nullability:`

## Bounds classes enforced in practice
1. Static bounds
- literal index validation against declared array lengths.

2. Dynamic bounds
- dynamic indices audited by compiler diagnostics and guarded by runtime list/array flows.

3. Container-aware bounds
- `array<T, N>`, `memRegion<T>`, list operations and indexed writes.

## Contract diagnostics in active use
Representative diagnostics:

- `E_TYPE_ARRAY_BOUNDS_001`
- `E_TYPE_MEMREGION_BOUNDS_001`
- `E_ARRAY_INDEX_001`
- `E_TYPE_ARRAY_001`
- `E_TYPE_MISMATCH_001`
- `E_LIST_BOUNDS_001`
- `E_ARRAY_MOVE_BOUNDS_001`
- `E_REGION_MOVE_BOUNDS_001`
- `E_TYPE_FIELD_MISSING_001`
- `E_TYPE_FIELD_001`
- `E_TYPE_FIELD_DUPLICATE_001`
- `E_TYPE_LAYOUT_001`
- `E_TYPE_BOUNDS_001`
- `E_TYPE_CONSTRUCTURE_001`
- `E_TYPE_DESTRUCTURE_001`
- `E_TYPE_COPY_001`
- `E_TYPE_CLONE_001`
- `E_TYPE_MOVE_001`
- `E_TYPE_INVARIANT_001`
- `E_TYPE_FFI_REPR_001`
- `E_TYPE_BINARY_LAYOUT_001`

## Binary-layout safety rule
`type` contracts declaring binary layout reject unstable fields:

- unstable examples: `text`, `list<T>`, `dinArray.unbinary<T>`, unknown custom unresolved forms,
- accepted core examples: integer families, bool, char, addr, fixed arrays of stable element types.

## Runtime boundary (explicit, non-silent)
This release forbids silent fallback for nominal `destructure/constructure` statement execution:

- runtime emits `E_TYPE_CONTRACT_DEFERRED_001` with `phase=runtime.type` and `status=deferred`,
- parser must never regress to `section not allowed outside [@primitive] context` for `type:` clauses.

## Evidence commands
```bash
cargo run -p ncc-cli -- compiler types examples/expert_release_v2/types/01_type_contract_release.n --json
cargo run -p ncc-cli -- compiler bounds examples/expert_release_v2/runtime/11_runtime_arrays_lists_memory_release.n --json
cargo test -p ncc-cli --test official_arrays_lists_indexing_matrix -- --nocapture
cargo run -p ncc-cli -- check examples/expert_release_v2/types/01_type_contract_release.n
```

## Expert suite link
- `examples/expert_release_v2/types/01..10`
- `examples/expert_release_v2/runtime/11..20`
- `examples/expert_release_v2/ffi/21..30`

All 30 files are between 50 and 250 lines and pass `ncc check` in this release validation tranche.
