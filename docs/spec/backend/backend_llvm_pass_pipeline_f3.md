# Backend LLVM/MCT F3: pass pipeline and optimizer ownership

Status: closed for MCT-owned LLVM optimizer pipeline selection and LLVM
new-pass-manager application.

F3 gives the LLVM adapter an explicit optimizer contract owned by `ncc-mct`.
The adapter no longer treats optimization as an implicit host flag. It carries a
typed pass-pipeline record in `MctLlvmModuleIR`, reports that pipeline through
JSON, and applies the selected optimization level through LLVM's new pass
manager by using `opt -passes=<pipeline>` and the Clang driver.

## Closed Surface

- `MctLlvmOptimizationLevelIR`: `O0`, `O1`, `O2`, `O3`, `Os`, `Oz`.
- `MctLlvmPassIR`: pass name, stage, purpose and enabled state.
- `MctLlvmPassPipelineIR`: owner, profile, Clang flag, `opt` pipeline string,
  pass list, diagnostics and parity boundary.
- `MctLlvmModuleIR.pass_pipeline`.
- `ncc mct pass-pipeline --json --opt-level <level>`.
- `ncc mct emit-llvm ... --opt-level <level>`.
- `ncc mct llvm-smoke ... --opt-level <level>`.
- `ncc mct llvm-exe-smoke ... --opt-level <level>`.
- `ncc mct llvm-pipeline-smoke ... --lto <mode> --pgo <mode>`.
- ThinLTO and full-LTO compile smoke.
- PGO generate/run/profraw/merge/use smoke.
- DWARF v5/debug-for-profiling metadata preservation.

## Pipeline Ownership

The owner is `ncc-mct`. The selected level is deterministic and maps to:

- `O0`: validation/officialization only.
- `O1`: scalar promotion and local simplification.
- `O2`: scalar, CFG, loop, memory and vectorization passes.
- `O3`: O2 plus loop-unroll and aggressive scalar combination.
- `Os`: size-oriented scalar/CFG/memory pipeline.
- `Oz`: tiny-size pipeline with the most conservative size policy.

The pipeline records both:

- `clang_opt_arg`, used by object/executable smoke;
- `opt_passes_arg`, the official new-pass-manager string for hosts that
  provide `opt`.

## Closure

F3/F8 now close deterministic pipeline ownership and real application of that
pipeline through installed Clang/LLVM tools:

- `opt -passes=<pipeline>` executes the LLVM new pass manager.
- `clang -O* -x ir` applies the same selected optimization level.
- `clang -flto=thin` and `clang -flto=full` validate both LTO modes.
- `clang -fprofile-generate`, the generated executable, `llvm-profdata merge`
  and `clang -fprofile-use` validate PGO end to end.
- `-g -gdwarf-5 -fdebug-info-for-profiling` preserves debug metadata through
  the pipeline.

Embedding `libLLVM` in-process is not required for this closed CLI contract.
If `opt`, `clang` or `llvm-profdata` is absent, the smoke commands report a
missing tool and do not fake pass evidence.

## Evidence

```sh
ncc mct pass-pipeline --json --opt-level 2
ncc mct emit-llvm examples/backend/mct_llvm_pass_pipeline.n \
  --out target/tmp/mct-llvm/mct_llvm_pass_pipeline.ll \
  --cc nC.sysv64 --opt-level 3 --json
ncc mct llvm-smoke examples/backend/mct_llvm_pass_pipeline.n \
  --out-dir target/tmp/mct-llvm/pass-pipeline \
  --cc nC.sysv64 --opt-level 2 --json
ncc mct llvm-exe-smoke examples/backend/mct_llvm_pass_pipeline.n \
  --out-dir target/tmp/mct-llvm/pass-pipeline-exe \
  --cc nC.sysv64 --opt-level 2 --json
ncc mct llvm-pipeline-smoke examples/backend/mct_llvm_pass_pipeline.n \
  --out-dir target/tmp/mct-llvm-f8 \
  --cc nC.sysv64 --opt-level 2 --lto both --pgo generate-use --json
```
