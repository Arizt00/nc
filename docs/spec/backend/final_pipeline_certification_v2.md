# Final pipeline certification v2

Status: normative.

## Authority

`tools/certification/final-target-matrix.v1.json` is the structured authority
for final compiler certification. `ncc-certification-v43 certify-final-pipeline`
consumes that matrix and emits an immutable evidence report. The legacy
`certify-raw-optimizer` command is only a compatibility entry point to the
same native implementation. Build and executable paths follow the
[V43 native tool instructions](../../NATIVE_TOOLS_V43_COMMANDS.md).

Every target route must pass all material stages:

1. parser (`emit-ast`);
2. semantic analysis (`check`);
3. typed IR (`emit-ir`);
4. build orchestration;
5. target LLVM IR;
6. per-source machine IR and `machine check`;
7. object emission with target-format magic;
8. internal object verification;
9. target-aware disassembly;
10. final link artifact.

A passing file-existence check cannot hide a failed validator. The route fails
when any mandatory stage fails, even if another stage of the same class passed.

## Target surface

The matrix contains one authoritative route for each declared target:

- hosted x86_64 on Linux, Windows and macOS;
- hosted AArch64 on Linux and macOS;
- hosted i386 Linux and RISC-V64 Linux;
- WASI and freestanding WASM32;
- freestanding x86_64, i386, AArch64, ARMv7-A, ARMv7E-M, RV32IMAC and RV64.

The standard WASM route retains the generated module, target LLVM IR and a
source-specific validated `.nm`. Bare-metal RISC-V targets use the kernel route;
there is no duplicate standard route without a boot contract.

## Execution classes

Execution evidence is classified and never substituted:

- `native`: the exact linked artifact runs on its host architecture;
- `qemu-user` or `wasmtime`: the exact target program runs under the declared
  user-mode runtime;
- `qemu-system`: the boot artifact starts with the command retained by the
  build manifest and emits the ordered serial oracle;
- `physical_hardware`: a board manifest flashes the exact linked SHA-256,
  resets the board and captures the ordered hardware oracle.

QEMU commands are argv arrays executed without a shell. Serial and firmware
placeholders are resolved explicitly. An unresolved placeholder, absent
firmware or missing runner is visible and cannot become a pass.

## Stress authority

The raw-memory stress bundle executes these exact scenarios in order:

- `heap-realloc`;
- `fragmentation`;
- `concurrent-atomics`.

The kernel stress bundle executes:

- `vm-exit-storm`;
- `smp-contention`;
- `cow-page-faults`;
- `invalid-dma`;
- `simultaneous-interrupts`;
- `snapshot-restore`.

Each bundle is independently verified, including schema, scenario order,
checksums and SHA-256 inventory. Smoke, CI and endurance profiles differ only
in workload size, not in admitted scenarios.

## Physical certificate

Physical input uses `<route-id>=<board-manifest>`. The certifier replaces the
manifest artifact path and hash with the exact route output before calling
`ncc machine certify-board`. A physical pass requires:

- `status=certified`;
- `physical_board=true`;
- `execution_class=physical_hardware`;
- the exact route target and linked-artifact SHA-256;
- a passing `verify-board-certificate` result.

Validation-only bundles, emulators, absent probes and hash mismatches fail a
required physical lane. Optional HMAC attestation follows the board certificate
specification.

## Reports

Each run retains `report.json`, `report.md`, stage stdout/stderr, serial logs,
objects, disassemblies, linked artifacts and `SHA256SUMS`. The final states are:

- `certified`: all selected material and execution requirements passed, plus
  every selected physical route when one is required;
- `software_certified_execution_partial`: material and stress passed but at
  least one required runner or physical board did not execute;
- `failed`: a material or stress requirement failed.

`--require-exact-head`, `--require-execution` and `--require-physical` promote
their corresponding partial states to command failures.

## Import rule

`print` and `println` are not implicit prelude functions. Certification sources
that use them must keep an explicit output import. The certifier does not relax
parser or semantic rules for fixtures.
