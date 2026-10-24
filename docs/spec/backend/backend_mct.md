# MCT: Modal Container Translate

Status: infrastructure contract ready.

MCT means `Modal Container Translate`. It is the backend routing layer that lets
nC describe how a modal container can be translated through one of several
providers:

- `ncc-native`;
- LLVM;
- GCC;
- rustc.

MCT is not a new language mode and it does not change nC semantics. It is a
translation contract between lowered `ProgramIR` containers and backend
providers. The first closed provider is `ncc-native`. LLVM is now an active
adapter for a closed textual LLVM IR subset. GCC now has a C99 bridge adapter
for the declared scalar gate. rustc is a planned adapter for explicit
`extern C` / `repr(C)` boundaries.

## Why MCT Exists

Pointers, regions, FFI, SIMD and low-level memory contracts need a single place
where the compiler can answer:

- which modal container is being translated;
- which backend provider is allowed;
- which capabilities are required;
- which target triple is active;
- whether the provider is integrated or only a bridge;
- which diagnostics prevent silent fallback.

Without MCT, LLVM/GCC/rustc support would become scattered across parser,
semantic analysis, backend emission and package tooling. MCT keeps that routing
explicit.

## Providers

`ncc-native` is integrated and owns current backend/object artifacts. It also consumes
type-contract struct layout metadata for direct scalar struct-field native-fast lowering:
field reads, writes and `field++` become byte-offset stack addressing plus typed memory
loads/stores in the native scalar plan.

LLVM has an integrated adapter for textual LLVM IR module emission. It lowers
scalar integer/bool functions, internal calls, structured control-flow, pointer
loads/stores, pointer arithmetic, pointer difference and region pointer views to
real `.ll`. It also lowers fixed `array<T, N>` locals to LLVM stack arrays and
direct `type:` struct field reads/writes to named LLVM `%struct.*` layouts
using `getelementptr`, `load` and `store`. F2 adds dynamic view descriptors for
`array<T>`, `slice<T>`, `span<T>`, `memRegion<T>` and `dinArray<T>` families,
including backing storage for literals, descriptor length/capacity fields,
indexed load/store and `regionFromPtr<T>(ptr, len)` lowering. It can emit a
hosted `main` wrapper from `ref main ->> ...;`. `ncc mct
llvm-smoke` validates object emission with `clang` when the host provides it,
and `ncc mct llvm-exe-smoke` links and runs a hosted executable. F3 adds
MCT-owned pass-pipeline selection for `O0/O1/O2/O3/Os/Oz` and maps it to
Clang optimizer flags for LLVM smoke. F4 adds real LLVM debug metadata and
DWARF module flags, plus `clang -g` object/executable smoke. Full arbitrary
LLVM optimizer/backend/debugger parity is still not claimed.
F5 adds target-specific LLVM function attribute groups for the ncc official
target matrix.

GCC has an integrated C99 bridge for the declared scalar gate. It emits real C
from `ProgramIR`, supports scalar values, internal calls, structured control
flow, stdio print lowering and hosted `ref main` executable smoke through
`gcc`/`cc` when available. It also exposes struct layout and dynamic descriptor
metadata for the next aggregate bridge gate. Full GCC plugin/GIMPLE parity is
not claimed.

rustc is restricted to explicit FFI-safe boundaries. Rust internal ABI and MIR
backend parity are not claimed.

## Capabilities

MCT capabilities include:

- modal container translate;
- function IR;
- type layout;
- pointer ops;
- memRegion ops;
- control flow;
- SIMD;
- auto-vectorization;
- atomics/volatile;
- C FFI;
- C++ FFI;
- Rust extern C;
- object emit;
- executable link;
- debug/unwind;
- optimizer pipeline.

## CLI

```sh
ncc mct doctor --json
ncc mct manifest --json
ncc mct parity --json
ncc mct providers --list
ncc mct translate examples/helloWorld.n --json --cc nC.sysv64
ncc mct emit-llvm examples/backend/mct_llvm_scalar_pointer.n --out target/tmp/mct-llvm/mct_llvm_scalar_pointer.ll --cc nC.sysv64
ncc mct llvm-smoke examples/backend/mct_llvm_scalar_pointer.n --out-dir target/tmp/mct-llvm --json --cc nC.sysv64
ncc mct llvm-exe-smoke examples/backend/mct_llvm_array_executable.n --out-dir target/tmp/mct-llvm --json --cc nC.sysv64
ncc mct llvm-exe-smoke examples/backend/mct_llvm_dynamic_array_slice.n --out-dir target/tmp/mct-llvm/dynamic-array-slice --json --cc nC.sysv64
ncc mct pass-pipeline --json --opt-level 2
ncc mct llvm-exe-smoke examples/backend/mct_llvm_pass_pipeline.n --out-dir target/tmp/mct-llvm/pass-pipeline-exe --json --cc nC.sysv64 --opt-level 2
ncc mct llvm-smoke examples/backend/mct_llvm_debug_dwarf.n --out-dir target/tmp/mct-llvm/debug-dwarf-smoke --json --cc nC.sysv64 --opt-level 2
ncc mct llvm-exe-smoke examples/backend/mct_llvm_target_attributes.n --out-dir target/tmp/mct-llvm/target-attrs-exe --json --cc nC.sysv64 --opt-level 2
ncc mct emit-gcc-c examples/backend/mct_gcc_scalar_bridge.n --out target/tmp/mct-gcc/scalar.c --json --cc nC.sysv64
ncc mct gcc-smoke examples/backend/mct_gcc_scalar_bridge.n --out-dir target/tmp/mct-gcc --json --cc nC.sysv64 --allow-missing-tools
```

`translate` parses, checks and lowers source to `ProgramIR`, then emits the MCT
translation report for each modal unit.

`parity` emits `nc.mct.declared_parity.v1`. It is the handoff gate from
LLVM/MCT work into GCC work: the declared ncc LLVM/MCT surface is closed, while
the arbitrary external LLVM universe remains outside the claim.

`emit-llvm` writes real textual LLVM IR. It is not JSON disguised as LLVM IR.

`llvm-smoke` writes `.ll`, invokes `clang -x ir -c`, and reports whether an
object file was created.

`llvm-exe-smoke` writes `.ll`, invokes `clang -x ir` to link a hosted
executable, runs that executable on the Linux host, and reports the exit code
and captured output.

`emit-gcc-c` writes real C99 bridge source for the declared GCC scalar and
dynamic aggregate gate.

`gcc-smoke` writes C, invokes `gcc` or `cc`, runs the executable when compile
succeeds, and reports missing tools honestly.

## Truth Boundary

MCT deliberately reports:

- `ncc_native_integrated = true`;
- `llvm_adapter_integrated = true`;
- `declared_llvm_mct_parity = true`;
- `gcc_ready_from_declared_parity = true`;
- `gcc_bridge_integrated_for_declared_scalar_dynamic_aggregate_pointer_region_aggregate_abi_subset = true`;
- `full_llvm_parity = false`;
- `full_llvm_external_universe_parity = false`;
- `full_gcc_parity = false`;
- `full_rustc_parity = false`.

This is the correct closure point for moving to GCC. The closed claim is
declared LLVM/MCT parity for ncc's current proven backend surface: provider
contracts, textual LLVM IR, aggregate/dynamic descriptor lowering,
vector-of-struct access, byte-stable print lowering, debug metadata, target
attributes, pass pipeline application, CPU tuning, toolchain smoke and honest
missing-tool reporting. Full arbitrary LLVM universe parity remains a separate
non-blocking claim.

F9 closes exact source-span parity for current LLVM-lowered statements, but
future language surfaces must still prove their own source-map coverage. Full
GCC/rustc parity remains gated by their own adapters.

The GCC bridge currently covers real C99 scalar/control-flow/print lowering,
the vector-of-structs descriptor surface (`data`, `len`, `cap`, `push`,
`packets[i].field`, `packets.get(i).field`) and the pointer/region family:
`ptrRead`, `ptrWrite`, `ptrOffset`, `ptrDiff`, `regionFromPtr` and
`regionPtrAt`. Aggregate parameters/returns for structs and dynamic descriptors
are emitted as C99 functions and delegated to the host C ABI.

F2 specifically closes dynamic arrays/slices for the local LLVM executable lane.
It does not claim a complete grow/realloc/push/pop runtime surface for
`dinArray`, nor a final cross-target descriptor ABI.

F3 specifically closes LLVM optimizer pipeline ownership for MCT. F8 then
closes application of that selected pipeline through Clang/`opt`, including
LTO, PGO and debug metadata. The remaining boundary is embedded in-process
`libLLVM` pass-manager API ownership.

F4 specifically closes real LLVM debug metadata emission for the MCT LLVM lane.
It emits DWARF metadata accepted by Clang. F9 adds exact statement source-map
line/column mapping for the current LLVM adapter surface.

F5 specifically closes target-specific LLVM function attribute selection for
the ncc official target matrix. It does not claim the full LLVM attribute
universe for every CPU microarchitecture.
