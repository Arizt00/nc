# ncc Optimizer Pipeline

Status: closed for the initial native-fast optimizer surface.

This document defines the first real performance optimizer pipeline for normal nC. It does not claim full C/C++/Rust optimizer parity yet. It closes a measurable release-fast path that can be inspected, benchmarked, and validated from the CLI.

## Scope

The optimizer pipeline applies to backend-lowerable normal nC programs built or run with the release-fast/native-fast path.

It covers:

- parser, semantic and type-contract preflight;
- BackendObjectIR materialization;
- object section, symbol and relocation statistics;
- ELF64 non-executable stack marking through `.note.GNU-stack`;
- hosted launcher C optimization flags;
- linker section garbage collection and non-executable stack flags when supported;
- native entry symbol resolution from `ref main ->> ...;` or explicit `--entry`;
- zero-cost runtime routing where the nC interpreter is not used.

It does not cover:

- global SSA optimization;
- register allocation beyond current backend lowering;
- cross-module LTO;
- machine evidence/seal changes;
- basal or machine semantic changes.

## CLI

```sh
ncc optimize examples/performance/native_fast/minimal_return.n --json
ncc optimize examples/performance/native_fast/minimal_return.n --out target/tmp/minimal.optimize.json
```

The JSON report has schema:

```json
{
  "schema": "ncc.performance.optimizer.v1",
  "status": "closed",
  "profile": "release-fast",
  "entry_symbol": "container.boot",
  "object_stats": {},
  "elf": {},
  "zero_cost_runtime": {},
  "passes": []
}
```

## Real Passes

`semantic_and_type_contract_preflight` rejects invalid programs before optimization. This prevents benchmark-only success paths and keeps release-fast tied to the real compiler pipeline.

`backend_object_stats` materializes real BackendObjectIR and counts sections, symbols, relocations, text bytes, machine bytes and metadata bytes.

`elf_non_executable_stack_note` writes a real `.note.GNU-stack` section in ELF64 output. GNU-compatible linkers use this to avoid executable-stack fallback behavior.

`hosted_launcher_optimized_c_flags` builds the minimal process-entry C launcher with `-O2`, function/data sections and reduced unwind table overhead where supported.

`linker_gc_sections_noexecstack` passes linker flags for unused-section collection and non-executable stack where supported.

`native_fast_zero_cost_dispatch` runs the backend-lowered entry through a hosted stub, not the nC runtime interpreter.

## Benchmark Integration

`ncc perf native-fast --json` embeds the optimizer report inside the nC build case. This makes each benchmark result explain which optimizer path produced the measured artifact.
