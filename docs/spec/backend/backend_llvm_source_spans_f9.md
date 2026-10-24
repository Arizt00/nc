# Backend LLVM/MCT F9: source-span parity

Status: closed for the current LLVM-lowered statement surface.

F9 removes the previous synthetic-only debug-location path for real CLI LLVM
emission. When `ncc mct emit-llvm`, `llvm-smoke`, `llvm-exe-smoke`,
`llvm-pipeline-smoke` or `llvm-toolchain-smoke` loads a source file, the CLI now
passes the source text into ProgramIR lowering. ProgramIR carries a sidecar
`source_map`; the LLVM adapter consumes that map and emits `DILocation` with the
real source line and column for each matched lowered statement.

## Closed Surface

- `SourceSpanIR`.
- `StatementSourceSpanIR`.
- `ProgramSourceMapIR`.
- `ProgramIR.source_map`.
- `lower_with_backend_profile_and_source(...)`.
- `attach_source_map_to_program_ir(...)`.
- `MctLlvmModuleIR.source_map`.
- `MctLlvmDebugMetadataIR.source_span_parity`.
- CLI JSON report for source-map entries and exact/missing/synthetic counts.
- LLVM `DILocation(line, column)` from source-map entries.
- Tests proving source lines in `.n` fixtures reach LLVM debug metadata.
- Escaped string literal source matching: the locator compares both the raw
  source line and a decoded-escape view, so `"line\n"` can map exactly to the
  decoded IR literal containing a real newline byte.
- Optimizer debugger UX handoff: optimized transformation categories now emit
  `debugger_ux` records that point back to this source-map policy instead of
  silently losing stepping evidence.

## Policy

The source map is authoritative for the current LLVM adapter. If a lowered
statement cannot be matched to source text, the module reports
`partial_source_span_parity`; it does not silently claim exact parity. Calls that
lower ProgramIR without source text keep the explicit
`synthetic_source_spans_unavailable` status.

## Evidence

```sh
ncc mct emit-llvm examples/backend/mct_llvm_debug_dwarf.n \
  --out target/tmp/mct-llvm-f9/mct_llvm_debug_dwarf.ll \
  --cc nC.sysv64 --opt-level 2 --json
```

Expected JSON fields:

- `source_map.status = exact_source_span_parity`.
- `source_map.missing_span_count = 0`.
- `debug_metadata.source_span_parity = true`.

Expected LLVM IR:

- `!DILocation(line: <source line>, column: <source column>, scope: ...)`.
- Escaped text literals in LLVM constants use real escaped bytes, for example
  `\0A`, `\22`, `\5C`, `\00` and UTF-8 emoji bytes, while the source map still
  reports `exact_source_span_parity`.

## Truth Boundary

Closed:

- exact source-span propagation for statements currently represented in
  `StatementIR` and lowered by the LLVM adapter;
- exact line/column debug locations for that surface;
- JSON visibility for partial or missing span coverage.

Not claimed:

- global Clang/rustc frontend parity;
- Clang frontend parity;
- rustc frontend parity;
- future constructs outside the current `MctLlvmModuleIR.construct_coverage`
  catalog;
- bit-for-bit native debugger behavior parity with every Clang/rustc optimized
  build mode.

Any future statement lowered to LLVM must either attach an exact source-map entry
or be reported as partial/missing evidence.
