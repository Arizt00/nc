# Backend LLVM/MCT F4: debug metadata and DWARF

Status: closed for real LLVM debug metadata emission in the MCT LLVM lane.

F4 gives the LLVM adapter a concrete DWARF path. The generated `.ll` is no
longer only executable IR; it now carries LLVM debug metadata nodes that Clang
accepts with `-g` during object and executable smoke.

## Closed Surface

- `MctLlvmDebugConfigIR`: source file, directory, producer, DWARF version,
  debug info version and emission kind.
- `MctLlvmDebugMetadataIR`: structured report of compile unit, file,
  subprogram, local variable, location and type metadata.
- `MctLlvmModuleIR.debug_metadata`.
- `!llvm.dbg.cu`.
- `!llvm.module.flags` with `Dwarf Version` and `Debug Info Version`.
- `!llvm.ident`.
- `!DIFile`.
- `!DICompileUnit`.
- `!DISubprogram` per emitted LLVM function.
- `!DILocation` on emitted instructions.
- `!DILocalVariable` and `llvm.dbg.declare` for local slots and parameters.
- Basic debug types for integers, bools, pointers, arrays and current struct
  composites.
- CLI JSON reporting for debug metadata.
- Clang smoke integration with `-g` when debug metadata is enabled.
- Optimizer UX handoff is closed by
  `backend_optimizer_debugger_ux_parity.md`: every current optimized
  transformation category emits a debugger-facing explanation, while DWARF
  remains the carrier for source locations.

## Source Mapping Policy

F4 introduced the real DWARF metadata surface. F9 closes the next layer for the
current LLVM-lowered statement surface: `ProgramIR.source_map` carries statement
source spans, and LLVM `DILocation` consumes those real line/column values when
the CLI lowers from a source file. Calls that lower ProgramIR without source text
still report `synthetic_source_spans_unavailable` instead of silently claiming
source parity.

## Evidence

```sh
ncc mct emit-llvm examples/backend/mct_llvm_debug_dwarf.n \
  --out target/tmp/mct-llvm/mct_llvm_debug_dwarf.ll \
  --cc nC.sysv64 --opt-level 2 --json

ncc mct llvm-smoke examples/backend/mct_llvm_debug_dwarf.n \
  --out-dir target/tmp/mct-llvm/debug-dwarf-smoke \
  --cc nC.sysv64 --opt-level 2 --json

ncc mct llvm-exe-smoke examples/backend/mct_llvm_debug_dwarf.n \
  --out-dir target/tmp/mct-llvm/debug-dwarf-exe \
  --cc nC.sysv64 --opt-level 2 --json
```

## Truth Boundary

Closed:

- real LLVM debug metadata nodes;
- DWARF module flags;
- local variable debug declarations;
- Clang `-g` object/executable smoke;
- exact source line/column mapping through F9 for current LLVM-lowered
  statements;
- structured JSON report.

Not claimed:

- source-span parity for future constructs that have not yet entered the
  current LLVM construct catalog;
- full target-specific debug/unwind personality modeling;
- bit-for-bit interactive debugger parity with every Clang/rustc optimized
  build mode;
- embedded LLVM pass-manager ownership beyond the F3 selected pipeline.
