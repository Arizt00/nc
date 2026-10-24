# Backend MCT CPU Tuning Evidence F12

F12 connects the F11 MCT CPU tuning matrix to two concrete evidence paths:

1. Tuned LLVM object emission through external LLVM-compatible tools.
2. ncc-native SIMD scheduler row selection for the same declared CPU profile.

This phase does not claim the full LLVM CPU model universe or full native scheduler parity. It closes the declared bridge for the official ncc/MCT microarchitecture rows.

## CLI

```text
ncc mct cpu-tuning-smoke <file.n> \
  --out-dir <dir> \
  --cc <profile> \
  --cpu <microarch> \
  --tune-profile <debug|baseline|latency|throughput|size> \
  --opt-level <0|1|2|3|s|z> \
  --json
```

The matrix smoke runs the official object/disasm rows for the closed F12
evidence set:

```text
ncc mct cpu-tuning-matrix-smoke <file.n> \
  --out-dir <dir> \
  --all-targets \
  --opt-level <0|1|2|3|s|z> \
  --json
```

The current official matrix is:

- `nC.sysv64` + `intel-skylake` + `throughput`
- `nC.win64` + `intel-skylake` + `throughput`
- `nC.aapcs64` + `apple-m1` + `throughput`

Each row gets its own output subdirectory and reports the nested
`cpu-tuning-smoke` payload, including object/disassembly paths, hashes,
LLVM stage statuses and the ncc-native scheduler bridge status.

Each row also reports `native_selection_policy`. That field is the concrete
F12 selector contract between the MCT CPU row and ncc-native SIMD lowering:

- `microarchitecture_id` and `fine_tuning_profile_id` name the exact MCT row.
- `preferred_vector_width_bits` records the profile-selected vector width.
- `decisions` contains physical operation choices with operation, vector type,
  required feature, SIMD lane, selected mnemonic, selected assembly bytes,
  scheduler row and selection reason.
- `full_native_scheduler_parity` remains `false`; the policy is closed for the
  declared rows and operations, not for every future scheduler case.

The command writes:

- `<stem>_<cpu>_<profile>.ll`
- `<stem>_<cpu>_<profile>.bc`
- `<stem>_<cpu>_<profile>.opt.bc`
- `<stem>_<cpu>_<profile>.o`
- `<stem>_<cpu>_<profile>.disasm.txt`

The command records SHA-256 hashes for the object and disassembly when those artifacts exist.

## External Tool Evidence

The smoke path uses real host tools:

- `llvm-as`
- `opt`
- `llc`
- `llvm-objdump`

Missing tools are reported as `mct_cpu_tuning_object_disasm_missing_tools`. They are not counted as passing evidence.

`llc` receives:

- `-mtriple=<target triple>`
- `-mcpu=<F11 target cpu>`
- `-mattr=<F11 target features>`
- `-filetype=obj`

`llvm-objdump -d` disassembles the emitted object bytes.

## Native Scheduler Bridge

F12 maps declared MCT CPU rows to the current ncc-native SIMD scheduler model:

- `x86_64-v1`, `x86_64-v2` -> `x86-generic`
- `intel-skylake`, `intel-icelake-client` -> `x86-intel-skylake`
- `amd-znver3`, `amd-znver4` -> `x86-amd-zen3`
- `aarch64-generic`, `apple-m1`, `apple-m2` -> `aarch64-generic`
- `neoverse-n1`, `neoverse-v1` -> `aarch64-neoverse-n1`

`wasm32-generic` is a known CPU tuning row but has no native SIMD scheduler bridge in this phase.

The bridge filters SIMD schedule rows by target and by declared feature support. A CPU profile cannot inherit unsupported SIMD features merely because it shares the ISA family.

## Native Selection Policy

F12 now connects the bridge to concrete selector decisions through:

- `BackendNativeCpuTuningDecisionIR`
- `BackendNativeCpuTuningPolicyIR`
- `backend_native_cpu_tuning_policy_for_mct_cpu()`
- `select_backend_simd_instruction_for_mct_cpu()`

For `intel-skylake + throughput`, the policy selects 256-bit x86 rows where the
declared features allow them. Integer vector add and multiply select AVX2
mnemonics such as `vpaddd`/`vpmulld`; vector-memory gather selects the AVX2
gather row; AVX/SSE boundary cleanup selects `vzeroupper`. The same policy does
not silently upgrade plain Skylake to AVX-512 scatter because that CPU row does
not declare `+avx512f`.

For `apple-m1 + throughput`, the policy selects 128-bit AArch64 NEON rows such
as `add v0.4s` and `mul v0.4s`. It does not claim hardware gather/scatter for
Apple M1 because the declared MCT row has `supports_gather_scatter=false`.

The selector is intentionally data-driven by the MCT CPU row and fine tuning
profile. It does not change the default `FunctionBackendPlanIR` path unless the
caller explicitly asks for tuned CPU/profile reporting or tuned LLVM emission.

## Truth Boundary

Closed:

- MCT CPU tuning object/disasm smoke command.
- MCT CPU tuning object/disasm matrix smoke command.
- `llvm-objdump` discovery.
- object and disasm SHA-256 evidence fields.
- bridge from F11 CPU rows to ncc-native SIMD scheduler rows.
- native CPU tuning selection policy with concrete SIMD instruction decisions
  for the declared x86_64 and AArch64 F12 rows.
- selected bridge report embedded in `ncc mct cpu-tuning --json`.
- selected native policy report embedded in `ncc mct cpu-tuning --json`,
  `cpu-tuning-smoke`, `cpu-tuning-matrix-smoke` and `mct manifest`.
- manifest visibility through `cpu_tuning_f12_object_disasm_matrix`.

Not claimed:

- full LLVM CPU model universe.
- full LLVM scheduler itinerary parity.
- full ncc-native scheduler parity for every future scalar/vector operation.
- auto-vectorizer parity for arbitrary language patterns.
- global microarchitecture scheduling parity beyond the declared F12 rows.
- link or execution for the F12 object/disasm matrix.
- sustained multi-host CI evidence for every row.
