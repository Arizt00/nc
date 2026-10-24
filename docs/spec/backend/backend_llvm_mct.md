# Backend LLVM MCT Adapter

Status: adapter integrated for the closed subset.

The LLVM path is implemented through MCT, not by changing nC language
semantics. The adapter receives lowered `ProgramIR`, selects eligible regular,
root and security functions, and emits real textual LLVM IR. It refuses
primitive and machine authority surfaces because those modes remain native
authority surfaces.

## Closed Subset

The current adapter emits:

- module header with target triple and data layout;
- scalar integer and bool function signatures;
- local slots with `alloca`, `load` and `store`;
- fixed `array<T, N>` local storage as `[N x T]`;
- fixed `stc` arrays without explicit initializers as MCT `stc.array.zero_fill`
  surface descriptors with real storage byte counts;
- array literal materialization into LLVM stack storage;
- dynamic array read/write through `getelementptr`, `load` and `store`;
- `type:` struct layout emission as named LLVM `%struct.*` types;
- struct field ordering, field index, natural offset, size and alignment
  reporting;
- direct `@aggregate.field` reads and writes through LLVM
  `getelementptr inbounds`, `load` and `store`;
- internal function calls for known nC functions;
- scalar projections and returns;
- unary and binary integer operations;
- byte-sized `char` literals as scalar values;
- deterministic compact post-increment (`i++`, including `i = i++;`) as a
  typed load/add/store sequence over the target integer slot;
- field post-increment (`obj.field++`) as field `load/add/store`, returning
  the old field value in the expression result;
- pointer post-increment read (`*p++`) as old-pointer load, pointee load,
  byte-stride `getelementptr` and advanced-pointer store;
- comparisons and boolean logic;
- `if`, canonical `else if`, compact `elseIf` and `else` through one nested CFG;
- `while`;
- `do while`;
- classic `for`;
- range `for`, with start/end/step evaluated once, automatic ascending or
  descending direction when `step` is omitted, and a zero-step trap;
- `loop`;
- `repeat until`;
- `repeat count`, with its count evaluated once and a negative-count trap;
- indexed lowering for `for-in`, `same-in` and `each ... do ... while`,
  including the post-condition path after `continue`;
- labeled and unlabeled `break` and `continue` with loop-specific targets;
- `ptrRead<T>` as LLVM `load`;
- `ptrWrite<T>` as LLVM `store`;
- `ptrOffset<T>` as byte `getelementptr`;
- `ptrDiff<T>` as `ptrtoint`, subtraction and element-size division;
- `regionFromPtr` and `regionPtrAt` as pointer views;
- runtime render intrinsics (`renderInt`, `renderUInt`, `renderBinary`,
  `renderBin`, `renderFloat`, `renderBool`, `renderChar`, `renderType`,
  `renderDebug` and `renderStable`) as real UTF-8 byte materialization through
  stack literals, `select`, `snprintf`, and explicit binary digit byte stores;
- declared scalar `ft64` math lowering for `sqrtFloat`, `expFloat`,
  `sinFloat`, `cosFloat`, `logFloat` and `atan2Float`;
- MCT FP optimizer folding for finite literal FP arithmetic, comparisons,
  constants and declared math calls, with automatic pruning of now-unused LLVM
  FP intrinsic/libm declarations;
- dynamic `array<T>` as a two-word `{ptr, len}` descriptor with real backing
  storage for literals;
- `slice<T>`, `span<T>` and `memRegion<T>` as two-word descriptors;
- `dinArray.*<T>` as a three-word `{ptr, len, cap}` descriptor;
- dynamic view indexed load/store and `.len()` lowering;
- `dynamic_views` JSON report entries;
- MCT-owned LLVM pass pipeline selection for `O0`, `O1`, `O2`, `O3`, `Os`
  and `Oz`;
- Clang optimizer flag mapping for object/executable smoke;
- LLVM debug metadata emission with `DICompileUnit`, `DISubprogram`,
  `DILocation`, `DILocalVariable`, `llvm.dbg.declare`, DWARF module flags and
  `clang -g` smoke;
- target-specific LLVM function attribute groups with target CPU, target
  features, frame-pointer, stack and unwind policy for official target specs;
- F10 official target matrix reporting for `nC.sysv64`, `nC.win64`,
  `nC.aapcs64`, `nC.portable` and `nC.sovereign`;
- F11 CPU tuning matrix reporting with declared x86_64, AArch64 and wasm32
  microarchitecture rows;
- F11 fine tuning profiles for `debug`, `baseline`, `latency`, `throughput`
  and `size`;
- `ncc mct cpu-tuning --json` and tuned `emit-llvm --cpu ... --tune-profile
  ...` target attributes;
- `ncc mct cpu-tuning-smoke` object/disassembly evidence through
  `llvm-as`, `opt`, `llc` and `llvm-objdump`;
- `ncc mct cpu-tuning-matrix-smoke` object/disassembly matrix evidence for
  official SysV64, Win64 and AArch64 CPU tuning rows;
- F12 native scheduler bridge from declared MCT CPU rows to ncc-native SIMD
  scheduler rows;
- F12 native selection policy that chooses concrete SIMD instructions and
  schedules for declared CPU/profile rows without claiming unsupported ISA
  features;
- tuned `llvm-smoke` and `llvm-exe-smoke` object/executable paths when
  `--cpu ... --tune-profile ...` is provided;
- Clang/LLVM new pass-manager application through `opt -passes=<pipeline>`;
- ThinLTO and full-LTO compile smoke through Clang;
- PGO generate/merge/use smoke through Clang and `llvm-profdata`;
- debug-for-profiling and DWARF v5 flag propagation through the pipeline;
- executable `main` wrapper when the source declares `ref main ->> ...;`.

Unsupported function bodies are rejected with diagnostics. They are not emitted
as fake LLVM bodies.

Runtime render lowering is closed for direct intrinsic materialization and
printing/projection through the LLVM adapter. Typed `str + str` is also a
material expression lowering surface: LLVM emits stable NUL-terminated literal
globals and heap-backed concatenation. This does not claim implicit conversion
of numeric values or arbitrary opaque pointers to text.

## CLI

```sh
ncc mct emit-llvm examples/backend/mct_llvm_scalar_pointer.n \
  --out target/tmp/mct-llvm/mct_llvm_scalar_pointer.ll \
  --cc nC.sysv64

ncc mct llvm-smoke examples/backend/mct_llvm_scalar_pointer.n \
  --out-dir target/tmp/mct-llvm \
  --json \
  --cc nC.sysv64

ncc mct llvm-exe-smoke examples/backend/mct_llvm_array_executable.n \
  --out-dir target/tmp/mct-llvm \
  --json \
  --cc nC.sysv64

ncc mct llvm-exe-smoke examples/backend/mct_llvm_struct_layout.n \
  --out-dir target/tmp/mct-llvm/struct-exe \
  --json \
  --cc nC.sysv64

ncc mct llvm-exe-smoke examples/backend/mct_llvm_dynamic_array_slice.n \
  --out-dir target/tmp/mct-llvm/dynamic-array-slice \
  --json \
  --cc nC.sysv64

ncc mct pass-pipeline --json --opt-level 2

ncc mct cpu-tuning --json --cpu intel-skylake --tune-profile throughput

ncc mct cpu-tuning-smoke examples/backend/mct_llvm_target_attributes.n \
  --out-dir target/tmp/mct-f12 \
  --cc nC.sysv64 \
  --opt-level 2 \
  --cpu intel-skylake \
  --tune-profile throughput \
  --json \
  --allow-missing-tools

ncc mct cpu-tuning-matrix-smoke examples/backend/mct_llvm_target_attributes.n \
  --out-dir target/tmp/mct-f12/cpu-tuning-matrix-smoke \
  --all-targets \
  --opt-level 2 \
  --json \
  --allow-missing-tools

ncc mct emit-llvm examples/backend/mct_llvm_target_attributes.n \
  --out target/tmp/mct-f11/tuned.ll \
  --cc nC.sysv64 \
  --opt-level 2 \
  --cpu intel-skylake \
  --tune-profile throughput \
  --json

ncc mct parity --json

ncc mct llvm-exe-smoke examples/backend/mct_llvm_pass_pipeline.n \
  --out-dir target/tmp/mct-llvm/pass-pipeline-exe \
  --json \
  --cc nC.sysv64 \
  --opt-level 2

ncc mct llvm-smoke examples/backend/mct_llvm_debug_dwarf.n \
  --out-dir target/tmp/mct-llvm/debug-dwarf-smoke \
  --json \
  --cc nC.sysv64 \
  --opt-level 2

ncc mct llvm-exe-smoke examples/backend/mct_llvm_target_attributes.n \
  --out-dir target/tmp/mct-llvm/target-attrs-exe \
  --json \
  --cc nC.sysv64 \
  --opt-level 2

ncc mct llvm-toolchain-smoke examples/backend/mct_llvm_target_attributes.n \
  --out-dir target/tmp/mct-llvm-f6 \
  --all-targets \
  --json \
  --opt-level 2 \
  --allow-missing-tools

ncc mct llvm-pipeline-smoke examples/backend/mct_llvm_pass_pipeline.n \
  --out-dir target/tmp/mct-llvm-f8 \
  --json \
  --cc nC.sysv64 \
  --opt-level 2 \
  --lto both \
  --pgo generate-use

ncc-tools mct-smoke \
  --out-dir target/tmp/mct-llvm-ci/local \
  --json-out target/tmp/mct-llvm-ci/local/toolchain_smoke.json \
  --summary-md target/tmp/mct-llvm-ci/local/toolchain_smoke.md \
  --opt-level 2
```

`emit-llvm` writes `.ll`. If `--json` is passed it reports module metadata,
diagnostics and the selected pass pipeline. `llvm-smoke` writes `.ll`, invokes
`clang --target=<triple> -O* -x ir -c`, and reports object creation.
`llvm-exe-smoke` writes `.ll`, invokes `clang --target=<triple> -O* -x ir` to
link a hosted executable, then runs it when the target is the current Linux
host.
`llvm-toolchain-smoke` writes `.ll` for each selected target row and then uses
the explicit external LLVM-equivalent chain `llvm-as -> opt -> llc ->
lld-family`. Missing stage tools are reported as `missing_toolchain` and are
not counted as pass evidence.
`llvm-pipeline-smoke` is the F8 pipeline closure command. It runs `opt`, Clang
object emission with debug metadata flags, ThinLTO/full-LTO compilation, PGO
generation, `llvm-profdata merge` and PGO-use compilation.

`.github/workflows/mct-llvm-toolchain-multi-host.yml` runs the same retained
toolchain smoke on Linux x86_64, Linux AArch64, macOS AArch64 and Windows
x86_64 runners. Remote release evidence is only claimed after that workflow is
pushed and GitHub Actions artifacts are retained.

## Truth Boundary

Closed:

- LLVM IR emission for the subset above.
- CLI emission.
- CLI object smoke through `clang` when available.
- CLI executable link/run smoke through `clang` when available.
- Unit tests for scalar control flow and pointer intrinsic lowering.
- Unit tests for fixed arrays and `main` wrapper emission.
- Unit tests for struct layout and field access lowering.
- Unit tests for dynamic arrays, memRegion descriptors and constant OOB
  diagnostics.
- Unit tests for pass-pipeline ownership and module pipeline reporting.
- Unit tests for LLVM debug metadata and DWARF flags.
- Unit tests for MCT/LLVM FP optimizer folding and FP intrinsic declaration
  pruning.
- Unit tests for target attributes and official target matrix coverage.
- Unit tests for F10 official target matrix rows and the explicit non-claim of
  global LLVM microarchitecture/tuning parity.
- Unit tests for F11 CPU tuning rows, fine profiles, scheduling matrix, tuned
  LLVM attributes and CLI `emit-llvm` output.
- Unit tests for F12 native CPU tuning selection policy and manifest exposure.
- Unit tests for exact source-span parity on current LLVM-lowered statements.
- CLI surface and manifest for external LLVM toolchain matrix smoke.
- Honest missing-tool classification for `llvm-as`, `opt`, `llc` and
  LLD-family binaries.
- Multi-host CI workflow and retained evidence wrapper for the LLVM toolchain
  matrix.
- F8 Clang/opt pipeline application with LTO, PGO and debug metadata.
- Declared LLVM/MCT parity gate through `ncc mct parity --json`, readying the
  tree for GCC adapter work without claiming the arbitrary external LLVM
  universe.

Not claimed:

- complete aggregate ABI passing/returning across all targets;
- deep nested aggregate field paths;
- explicit `packed`/`repr` layout controls;
- optional embedded in-process LLVM pass-manager API hosting, if a future ncc
  driver wants to replace the closed `opt`/Clang execution path;
- full LLVM target-attribute universe beyond the ncc official target matrix;
- full LLVM microarchitecture and global tuning policy parity beyond the F11
  declared CPU tuning matrix;
- source-span conformance for language surfaces not yet lowered by the LLVM
  adapter;
- retained exact-head multi-host GitHub Actions evidence until the workflow is
  pushed and run.

Declared LLVM/MCT parity is closed for the current ncc surface. Full arbitrary
external LLVM universe parity remains outside that claim and is not a blocker
for starting the GCC adapter.
