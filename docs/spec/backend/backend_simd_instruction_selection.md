# Backend SIMD Instruction Selection

Status: closed for target-specific vector-expression SIMD lowering.

This document defines real SIMD instruction selection for vector expressions in
the ncc backend. It is deliberately byte-level: every selected row has a target,
feature, lane shape, operation, assembly spelling, expected native bytes and
disassembly needles. The lowering bridge maps real typed vector expressions in
IR to those rows rather than leaving SIMD as a disconnected table.

## Closed Targets

- x86_64 SSE.
- x86_64 SSE4.1.
- x86_64 AVX.
- x86_64 AVX2.
- x86_64 AVX-512F.
- AArch64 NEON.

## Closed Operations

The x86_64 SSE set covers:

- aligned `f32x4` load and store through `movaps`;
- `f32x4` add and multiply through `addps` and `mulps`;
- 128-bit bitwise `and`, `or` and `xor` through `pand`, `por` and `pxor`;
- `i32x4` add through `paddd`;
- `f32x4` shuffle through `shufps`.

The x86_64 SSE4.1 set covers:

- `i32x4` multiply through `pmulld`;
- `f32x4` immediate mask selection through `blendps`.

The machine encoder also materializes the legacy SIMD support rows used by
that profile: packed integer add/sub/mul/test, packed shifts, `pshufd`,
`shufps` and `blendps`, with reg-reg, reg-mem and immediate forms where the
instruction encoding defines them.

The x86_64 AVX set covers:

- aligned `f32x8` load and store through `vmovaps`;
- `f32x8` add and multiply through `vaddps` and `vmulps`;
- `f32x8` bitwise `and`, `or` and `xor` through `vandps`, `vorps` and `vxorps`;
- `f32x8` shuffle through `vpermilps`;
- `f32x8` mask selection through `vblendvps`;
- `vzeroupper` as the AVX/SSE transition boundary instruction.

The x86_64 AVX2/AVX-512F extension set covers:

- `i32x8` add and multiply through `vpaddd` and `vpmulld`;
- `f32x8` gather through `vgatherdps`;
- `f32x16` scatter through `vscatterdps`.

The AArch64 NEON set covers:

- `f32x4` load and store through `ldr q0` and `str q0`;
- `f32x4` add and multiply through `fadd v0.4s` and `fmul v0.4s`;
- `i32x4` add and multiply through `add v0.4s` and `mul v0.4s`;
- 128-bit bitwise `and`, `or` and `xor` through `and/orr/eor v0.16b`;
- shuffle through `rev64` and `tbl`;
- mask selection through `bsl`;
- lane gather/scatter through `ld1`/`st1`.

## Vector Expression Lowering

The closed lowering bridge recognizes:

- vector/simd/vec typed literals;
- binary vector `+`, `*`, `&`, `|` and `^`;
- official calls `vectorAdd`, `vectorMul`, `vectorAnd`, `vectorOr`,
  `vectorXor`, `vectorShuffle`, `vectorMaskSelect`, `vectorGather` and
  `vectorScatter`.

The selector is `select_backend_simd_instruction_encoding(target, feature, lane,
op)`. If a target/feature/lane/op combination is not declared, the backend does
not silently fall back to scalar code in this closure.

## Scheduling

Scheduling records are emitted for:

- generic x86_64;
- Intel Skylake;
- AMD Zen3;
- generic AArch64;
- Neoverse N1.

Each row carries latency, reciprocal throughput band, issue domain and a
scheduling note. Gather/scatter are modeled as high-cost vector-memory
operations; shuffle and mask-select are kept in their own pressure domains.

## Evidence Surface

The machine-readable manifest is:

```sh
ncc backend-model --industrial --json
```

The object/disasm smoke is:

```sh
ncc backend-model --industrial --simd-smoke --json
```

The smoke writes one assembly fixture per selected row under
`target/tmp/backend-simd-host-smoke`, assembles it with the target assembler,
checks the expected bytes inside the object file, disassembles with target
`objdump`, verifies mnemonic needles, and links a target artifact when the
external linker is installed.

Closure evidence on the WSL host:

- schema: `nc.backend.simd.host_smoke.v1`;
- status: `passed`;
- rows: 39;
- passed: 39;
- failed: 0;
- missing environment: 0.

## Auto-Vectorizer Bridge

The SIMD selector is now consumed by the backend auto-vectorizer for declared
loop patterns. `analyze_backend_auto_vectorization(function)` recognizes
contiguous map/zip, sum/product reductions, gather and scatter loops, then
selects one of the concrete SIMD rows described above. Rejected loops emit
diagnostics instead of silently falling back and claiming vectorization.

The auto-vectorizer manifest status is:

```text
closed_for_declared_loop_patterns_not_arbitrary_llvm_parity
```

## Boundaries

This closes the declared vector-expression SIMD lowering set and its
microarchitecture schedule records, plus the declared auto-vectorizer bridge. It
does not claim arbitrary-pattern auto-vectorizer parity, every
horizontal/widening/narrowing vector operation, or formal equivalence with
LLVM/GCC/rustc vector optimization.
