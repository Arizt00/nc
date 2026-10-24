# Backend LLVM/MCT F8: Clang Pipeline Ownership

Status: closed for Clang/LLVM new-pass-manager, LTO, PGO and debug metadata
application.

F8 closes the executable optimizer pipeline boundary for the LLVM MCT adapter.
The nC compiler owns the selected pipeline, emits the selected pipeline in
JSON, and applies it through installed LLVM tools:

- `opt -passes=<pipeline>` for explicit new pass manager execution;
- `clang -O* -x ir` for object/executable application;
- `clang -flto=thin` and `clang -flto=full` for LTO smoke;
- `clang -fprofile-generate` plus `llvm-profdata merge` and
  `clang -fprofile-use` for PGO smoke;
- `-g -gdwarf-5 -fdebug-info-for-profiling` for debug metadata preservation.

## Closed Surface

- `MctLlvmPassPipelineIR.clang_pass_manager_args`.
- `MctLlvmPassPipelineIR.clang_lto_args`.
- `MctLlvmPassPipelineIR.clang_pgo_generate_args`.
- `MctLlvmPassPipelineIR.clang_pgo_use_arg_template`.
- `MctLlvmPassPipelineIR.clang_debug_args`.
- LTO pass markers: `thinlto-prelink`.
- PGO pass markers: `instrprof-generate` and `instrprof-use`.
- Debug preservation marker: `debug-info-preservation`.
- CLI: `ncc mct llvm-pipeline-smoke`.
- Manifest: `llvm_pipeline_f8`.

## CLI Contract

```sh
ncc mct llvm-pipeline-smoke examples/backend/mct_llvm_pass_pipeline.n \
  --out-dir target/tmp/mct-llvm-f8 \
  --cc nC.sysv64 \
  --opt-level 2 \
  --lto both \
  --pgo generate-use \
  --json
```

The command writes textual LLVM IR, runs `opt`, compiles an object through
Clang with debug metadata flags, compiles ThinLTO/full-LTO objects, generates
instrumentation profile data, merges `.profraw` files into `.profdata`, and
recompiles with `-fprofile-use`.

If `clang`, `opt` or `llvm-profdata` is missing, the command reports
`mct_llvm_pipeline_smoke_missing_tools`. Missing tools are never counted as
pass evidence.

## Closure

Closed:

- full application of the selected LLVM new pass-manager pipeline through
  `opt`;
- full Clang-driver application of the selected optimization level;
- ThinLTO and full-LTO compile smoke;
- PGO generate/merge/use smoke;
- DWARF/debug-for-profiling flag propagation;
- JSON evidence for every stage.

The closed contract is the LLVM new pass manager as exercised by the installed
LLVM tools. `ncc-mct` owns the pipeline selection, `opt` executes the official
new-pass-manager string, and Clang applies the same optimization contract for
objects, LTO and PGO. Embedding `libLLVM` in-process is an implementation option
for a future driver architecture, not an open semantic or backend closure.

F8 therefore closes Clang/LLVM pipeline application without fake evidence. If a
host lacks `clang`, `opt` or `llvm-profdata`, the command reports missing tools
instead of claiming success.
