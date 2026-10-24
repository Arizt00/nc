# Exhaustive pattern matching

Status: closed across parser, sema, typed IR, LLVM and admitted native-fast aggregate lowering.

## Accepted patterns

- Wildcards and typed bindings.
- Scalar literals and inclusive or exclusive ranges.
- Struct and raw-union fields.
- Fixed arrays, nested fixed arrays and fixed-array rest bindings.
- Nominal enum variants with payload bindings or nested pattern syntax.
- Guards, including effectful boolean guards.

## Diagnostics

- Missing scalar intervals and enum variants are non-exhaustive.
- Fully covered later arms are unreachable.
- Guards do not contribute unconditional coverage.
- Raw unions require an explicit catch-all because they have no runtime discriminant.
- Arity, field and payload type mismatches are rejected before IR.

## Lowering

Scalar ranges become comparisons. Struct and array patterns become typed field/element reads. Nominal enums use the canonical tagged-union layout: `i32` tag followed by aligned payload storage. Bindings are materialized after a pattern succeeds and before its guard is evaluated.

The CLI executor uses the same pattern tree for runtime values. LLVM emission has explicit constructor, tag-read and payload-read constructs in the MCT coverage catalog.

## Native aggregate places

Nested enum payloads retain a typed aggregate place. Record fields lower as `payload offset + field-path offset`; fixed-array items lower as `payload offset + index * element size`, including record fields inside array elements. Native constructors copy aggregate payloads with their resolved size and alignment instead of treating them as scalar words.

Tests: `cargo test -p ncc-sema exhaustive_pattern_semantics_tests`, `cargo test -p ncc-ir exhaustive_pattern_ir_tests`, and `cargo test -p ncc-cli structural_pattern_runtime_tests --lib`. The IR suite retains a nested enum array/record `.n -> IR -> native bytes/ELF64` check and a Clang-linked execution smoke.
