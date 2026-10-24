# Machine Multi-Host Evidence Requirements

Machine Level 5 and Level 6 release evidence is split into local evidence and
retained external evidence.

## Local Evidence

Local evidence may prove:

- parser, semantic check, MachineIR, verifier, and encoding behavior;
- Linux x86_64 object link and execution on the local host;
- Linux x86_64 final ELF executable execution through the system loader;
- wasm32 module generation and Node execution when Node is available;
- object generation for ELF64, COFF, Mach-O, AArch64 ELF64, RISC-V ELF64, and
  wasm32.

Local evidence does not prove Windows or macOS execution, and it does not prove
aarch64 or riscv64 execution unless those targets run on a real runner or an
explicit emulator with retained logs.

## External Evidence

External release evidence requires GitHub Actions or equivalent retained CI
artifacts. The configured workflow is:

`.github/workflows/machine-level5-level6-release-industrial.yml`

The required retained artifact set is:

| Artifact | Host | Arch | Minimum evidence |
|---|---|---|---|
| `machine-ci-retained-windows-msvc-x86_64` | Windows | x86_64 | COFF object linked by the native MSVC Rust/link toolchain plus direct Phase 12 PE system-loader smoke with `.reloc`/ASLR and exit `42` |
| `machine-ci-retained-linux-gnu-x86_64` | Linux | x86_64 | GNU `gcc` link/harness smoke and final ELF system-loader smoke |
| `machine-ci-retained-linux-clang-x86_64` | Linux | x86_64 | Clang `clang` link/harness smoke and final ELF system-loader smoke |
| `machine-ci-retained-macos-clang-x86_64` | macOS | x86_64 | Clang Mach-O object/executable smoke and final Mach-O system-loader smoke |
| `machine-ci-retained-linux-gnu-aarch64` | Linux | aarch64 | Native Linux AArch64 runner execution with stdout `42` |
| `machine-ci-retained-linux-riscv64-qemu` | Linux | riscv64 via QEMU | `riscv64-linux-gnu-gcc` static link and `qemu-riscv64` execution with stdout `42` |
| `machine-ci-retained-aggregate` | Linux | aggregate | Fan-in validation that all six row artifacts include passing `machine_ci_evidence.json` |

## Required Retained Metadata

Each artifact must retain:

- workflow name;
- run id and run URL;
- job URL;
- host, architecture, toolchain and CI lane id;
- command transcript summary;
- produced artifact paths;
- artifact hashes;
- smoke stdout/stderr summary;
- system-loader executable path, hash, expected exit code, and observed exit code;
- pass/fail status.

Each row artifact must include:

- `reports/machine_closure/ci/<lane>/machine_ci_evidence.json`;
- `reports/machine_closure/ci/<lane>/machine_ci_evidence.md`.

The aggregate artifact must include:

- `reports/machine_closure/machine_ci_multi_host_retained_manifest.json`;
- `reports/machine_closure/machine_ci_multi_host_retained_manifest.md`.

The native `ncc-release verify-machine-ci` command now produces aggregate
schema `nC.machine.ci.retained.aggregate.v2`. Its digest covers compact UTF-8
JSON in insertion order with `evidence_hash` omitted; the encoding is recorded
in the report. Input file SHA-256 values are retained separately. Duplicate
lane IDs fail verification, and CI supplies `--head` for the exact commit.
This verifies retained records, not fresh hardware execution or signatures
of the externally referenced lane payloads.

## Claims Not Allowed Without External Evidence

Without retained external artifacts, the following claims are not allowed:

- `multi_host_verified`;
- `machine_level_6_closed`;
- `windows_executable_smoke_passed`;
- `macos_executable_smoke_passed`;
- `aarch64_execution_verified`;
- `riscv64_execution_verified`.

## Closure Rule

Machine Level 6 can be marked closed only when all six row artifacts and the
aggregate artifact exist for the exact head, and their JSON proves the expected
machine checks, object generation, link/executable smoke, system-loader smoke
where applicable, AArch64 native execution, and RISC-V/QEMU execution.
