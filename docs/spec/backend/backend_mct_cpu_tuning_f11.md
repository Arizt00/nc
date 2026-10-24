# Backend MCT CPU Tuning F11

Status: closed for the declared ncc/MCT CPU tuning matrix.

F11 closes the microarchitecture and fine tuning surface owned by ncc/MCT. The
goal is to make CPU selection, scheduling intent and LLVM target attributes
explicit and reproducible for the official backend lane. This is not a claim
that ncc now mirrors every CPU model, subtarget and scheduler itinerary known by
LLVM.

## Closed Surface

The matrix is exposed by `mct_cpu_tuning_matrix()` in `ncc-ir` and by:

```sh
ncc mct cpu-tuning --json
ncc mct cpu-tuning --list --target x86_64
ncc mct cpu-tuning --json --cpu intel-skylake --tune-profile throughput
```

`ncc mct manifest --json` also includes the matrix under `cpu_tuning_f11`.

## Microarchitecture Profiles

F11 closes these declared profiles:

| Id | Target | LLVM CPU | Vector width | Scheduling model |
| --- | --- | --- | --- | --- |
| `x86_64-v1` | x86_64 | `x86-64` | 128 | generic out-of-order x86 |
| `x86_64-v2` | x86_64 | `x86-64-v2` | 128 | generic x86-v2 |
| `intel-skylake` | x86_64 | `skylake` | 256 | Skylake port pressure |
| `intel-icelake-client` | x86_64 | `icelake-client` | 512 | Ice Lake AVX-512 |
| `amd-znver3` | x86_64 | `znver3` | 256 | Zen 3 balanced |
| `amd-znver4` | x86_64 | `znver4` | 512 | Zen 4 AVX-512 |
| `aarch64-generic` | aarch64 | `generic` | 128 | generic NEON |
| `apple-m1` | aarch64 | `apple-m1` | 128 | Apple M1 wide OOO |
| `apple-m2` | aarch64 | `apple-m2` | 128 | Apple M2 wide OOO |
| `neoverse-n1` | aarch64 | `neoverse-n1` | 128 | Neoverse N1 |
| `neoverse-v1` | aarch64 | `neoverse-v1` | 256 | Neoverse V1 SVE |
| `wasm32-generic` | wasm32 | `generic` | 128 | abstract wasm cost |

Each row records vendor/family, target features, preferred vector width, issue
width, reorder window, load latency, store-forward latency, branch penalty,
cache line size, FMA support, gather/scatter support and a named scheduling
model.

## Fine Tuning Profiles

F11 closes five tuning intents:

- `debug`: `O0`, preserve source shape and debug order.
- `baseline`: `O2`, balanced inlining, vectorization and scheduling.
- `latency`: `O2`, favor critical path and demand-driven behavior.
- `throughput`: `O3`, favor wide vectorization, ILP and port pressure.
- `size`: `Oz`, preserve code size and disable unprofitable expansion.

Every microarchitecture is crossed with every fine profile, producing 60
scheduling profiles. Each scheduling profile exposes concrete LLVM-style args:
`-O*`, `-mcpu=...`, `-mtune=...`, `-mattr=...` and
`-passes=default<...>`.

## LLVM Attribute Application

`ncc mct emit-llvm` accepts:

```sh
ncc mct emit-llvm examples/backend/mct_llvm_target_attributes.n \
  --out target/tmp/mct-f11/tuned.ll \
  --cc nC.sysv64 \
  --opt-level 2 \
  --cpu intel-skylake \
  --tune-profile throughput \
  --json
```

When the CPU target matches the backend profile, MCT emits tuned function
attributes:

- `target-cpu`;
- `tune-cpu`;
- `target-features`;
- frame pointer, stack and unwind attributes inherited from F5.

The selected fine tuning profile remains in MCT metadata/JSON fields
(`fine_tuning_profile`) and is not emitted as an LLVM string attribute.

If the selected CPU target does not match the backend target, the command fails
instead of silently retargeting the module.

The same `--cpu` and `--tune-profile` options are also accepted by
`llvm-smoke` and `llvm-exe-smoke`, so the selected attributes can flow into
Clang object and executable smoke paths when the host LLVM toolchain is
available.

## Truth Boundary

F11 explicitly reports:

- `full_llvm_cpu_universe = false`
- `full_llvm_scheduler_parity = false`

The closed contract is the declared ncc/MCT matrix above. LLVM contains many
more target CPUs, subtarget combinations, scheduling models, tuning flags and
target-specific optimizer details. Those can be added by future matrix rows, but
they are not claimed by F11.

## Validation

Coverage lives in `ncc-ir` and `ncc-cli`:

- `mct_cpu_tuning_matrix_closes_f11_profiles_without_global_llvm_claim`
- `mct_llvm_target_attributes_accept_cpu_tuning`
- `mct_cli_cpu_tuning_reports_selected_profile`
- `mct_cli_emit_llvm_accepts_cpu_tuning`

The validation checks matrix counts, declared CPU rows, fine tuning profiles,
LLVM argument construction, manifest exposure and real textual LLVM IR output
with tuned attributes.
