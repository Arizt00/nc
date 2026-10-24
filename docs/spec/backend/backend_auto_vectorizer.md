# Backend Auto-Vectorizer

Status: closed for declared loop-pattern auto-vectorization, not for arbitrary
LLVM/GCC/rustc parity.

The ncc backend now has a real auto-vectorizer analysis layer over the existing
SIMD instruction selector. It scans lowered `FunctionIR` loops, recognizes a
closed set of vectorizable loop shapes, selects concrete target SIMD rows, emits
cost/scheduling metadata, and reports rejected loops with diagnostics instead of
silently calling them vectorized.

## Closed Patterns

The closed pattern set is:

- vector expressions already present inside loops;
- contiguous array map;
- contiguous array zip with add, multiply and bitwise operations;
- sum reduction;
- product reduction;
- indexed gather;
- indexed scatter.

The entry point is:

```text
analyze_backend_auto_vectorization(function: &FunctionIR)
```

It returns `BackendAutoVectorizerReportIR` with:

- target;
- loops seen;
- vectorized loop rows;
- rejected loop rows;
- closed pattern list;
- `no_silent_scalar_fallback = true`;
- `arbitrary_pattern_parity = false`.

## Selected SIMD Rows

Every vectorized loop records the selected target row:

- target;
- feature;
- lane;
- SIMD operation;
- mnemonic;
- assembly spelling;
- native bytes;
- microarchitecture schedule.

The selector path is:

```text
loop pattern -> BackendSimdOpIR -> select_backend_simd_instruction_encoding
```

This means a loop is not marked vectorized unless it reaches a concrete SIMD
instruction row.

## Cost Model

Each vectorized loop records a small static cost model:

- scalar operations per iteration;
- vector operations per chunk;
- lanes per chunk;
- loop-depth weight;
- estimated speedup ratio.

This is intentionally conservative. It is enough for release reporting and for
rejecting unsupported patterns without claiming whole-program optimizer parity.

## Diagnostics

Current diagnostics:

- `E_AUTO_VEC_PATTERN_001`: the loop body did not match a closed pattern.
- `I_AUTO_VEC_VECTOR_EXPRESSION_001`: a typed vector expression in a loop was
  lowered to a SIMD row.
- `I_AUTO_VEC_ARRAY_PATTERN_001`: an array map/zip/gather/scatter pattern was
  lowered to a SIMD row.
- `I_AUTO_VEC_REDUCTION_001`: a sum/product reduction was lowered to a SIMD row.

## Evidence

Evidence fixtures:

```sh
cargo test -p ncc-ir auto_vectorizer --all-targets
cargo test -p ncc-cli backend_model_cli_tests --lib
ncc backend-model --industrial --json
```

The industrial backend manifest exposes:

```text
auto_vectorizer.status =
closed_for_declared_loop_patterns_not_arbitrary_llvm_parity
```

MCT consumes this boundary as a backend capability. `ncc-native` can expose the
declared auto-vectorizer immediately; LLVM/GCC/rustc providers must prove their
own adapter evidence before claiming parity.

## Boundary

This is not a declaration of total arbitrary-pattern auto-vectorizer parity with
LLVM, GCC or rustc. The current closure is a real vectorizer for declared loop
patterns. Unsupported loops are rejected or reported; they are not silently
treated as vectorized and they do not receive fake SIMD evidence.
