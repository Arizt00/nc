# Runtime Memory Move/Copy (v2.1.0)

Status: implemented-core with explicit guarded limits.

Implemented and validated in runtime executor:
- `arrayCopy`, `arrayMove`, `arraySwap`, `arrayFill`, `arrayZero`, `arrayReverse`.
- `regionCopy`, `regionMove`, `regionZero`, `regionFill`.
- `memoryCopy`, `memoryMove`, `memoryCompare`, `memoryZero`.
- `copy.value`, `copy.ref`, `copy.view`, `copy.deep`, `copy.binary`, `copy.unbinary`.
- `clone.value`, `clone.shallow`, `clone.deep`, `clone.region`, `clone.pointee`, `clone.binary`, `clone.unbinary`.
- Direct `copy x`, `clone x`, and `move x` transfer expressions.
- `move(@x)` runtime invalidation sentinel (`<moved>`) and use-after-move guard.

Pointer-object copy base:
- `@b >> T : @a;` copies a typed object bitwise/shallow by default. If `T` contains `ptr<T>`, `addr`, `site<T>`, or `memRegion<T>` fields, the pointer values are copied exactly; pointees are not cloned.
- `@b >> T : copy @a;` is the explicit shallow copy form. It respects `copy:` contracts and fails with `E_TYPE_COPY_DENIED_001` when a type contract denies shallow copy.
- `@b >> T : clone @a;` is the explicit deep clone request. Pointer-owning types must declare a deep clone contract, otherwise sema reports `E_TYPE_CLONE_001`; denied contracts report `E_TYPE_CLONE_DENIED_001`.
- `@b >> T : move @a;` transfers the value and marks `@a` as moved. Later static use reports `E_TYPE_MOVE_AFTER_MOVE_001`; runtime still preserves `E_MOVE_USE_AFTER_MOVE_001` for executed use-after-move paths.
- This is intentionally C/C++-grade: shallow copies can create aliases, shared stale state, and double-free hazards. nC permits that power by default and diagnoses only what can be proven or contractually forbidden.

Implemented diagnostics in this block:
- `E_MOVE_USE_AFTER_MOVE_001`
- `E_TYPE_MOVE_AFTER_MOVE_001`
- `E_TYPE_COPY_DENIED_001`
- `E_TYPE_CLONE_DENIED_001`
- `E_MOVE_TYPE_001`
- `E_COPY_UNSUPPORTED_001`
- `E_COPY_PTR_ALIAS_001`
- `E_CLONE_UNSUPPORTED_001`
- `E_CLONE_FN_CODE_001`
- `E_ARRAY_MOVE_BOUNDS_001`
- `E_REGION_MOVE_BOUNDS_001`
- `E_REGION_COPY_OVERLAP_001`
- `E_MEMORY_MOVE_ORIGIN_001`
- `E_MEMORY_MOVE_SIZE_001`
- `E_MEMORY_STALE_SITE_001`
- `E_SITE_ARRAY_ELEMENT_001`
- `E_SITE_LIST_ELEMENT_001`
- `E_SITE_STALE_001`
- `E_ADDR_ARRAY_UNAVAILABLE_001`
- `E_ADDR_LIST_UNSTABLE_001`
- `E_LIST_EXPORT_001`
- `E_LIST_EMPTY_001`
- `E_LIST_ALLOC_001`
- `E_LIST_ALIAS_001`
- `E_LIST_MOVED_001`

Validated via official matrix and examples:
- `crates/ncc-cli/tests/official_arrays_lists_indexing_matrix.rs`
- `examples/memory/array_move_copy.n`
- `examples/memory/region_move_copy.n`
- `examples/memory/memory_move_overlap.n`
- `examples/memory/copy_clone_region.n`
- `examples/pointers/ptr_copy_shallow_object_ok.n`
- `examples/pointers/ptr_copy_explicit_shallow_ok.n`
- `examples/pointers/ptr_clone_contract_deep_ok.n`
- `examples/pointers/ptr_move_object_ok.n`

Explicit current limits:
- stale-site generation lineage is implemented for heap pointer lifecycle, but full list/dinArray realloc stale lineage remains partial and tracked separately.
- parser accepts `clone.fn(...)` (keyword segment enabled), while broader keyword-segment dotted-call surface is still intentionally conservative.
