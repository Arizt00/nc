# Machine Target Admission Profiles

`ncc machine target-admission` defines the admission path for targets outside
the current hosted/native set.

Most profiles are not native targets yet. They are material contracts that must
be satisfied before `machine` can accept a GPU, DSP, firmware image or legacy
microcontroller target as real backend output.

Two 32-bit CPU profiles are now admitted as native machine targets:

- `riscv32imac-unknown-elf`: RV32IMAC, ILP32, ELF32 relocatable output,
  RV32 RELA relocations, QEMU link-smoke contract.
- `i386-unknown-linux-gnu`: i386 SysV cdecl, ELF32 relocatable output, i386
  REL relocations, QEMU or multilib link-smoke contract.

Required material axes for every profile:

- registers;
- ISA;
- instruction encoding;
- ABI;
- data layout;
- output format;
- relocations;
- linker/loader;
- execution evidence;
- corpus.

Current candidate profiles:

| Family | Profiles |
|---|---|
| GPU | `nvptx64-nvidia-cuda`, `amdgcn-amd-amdhsa` |
| DSP | `hexagonv68-unknown-elf` |
| 32-bit CPU | admitted: `i386-unknown-linux-gnu`, `riscv32imac-unknown-elf`; gated: `armv7em-none-eabi` |
| Firmware | `x86_64-unknown-none-flat`, `x86_64-uefi-pe` |
| Legacy microcontroller | `avr-atmega328p-none-elf`, `msp430-none-elf` |

Admission rule:

```text
ncc machine target-admission admit <profile>
```

must fail with `E_MACHINE_TARGET_PROFILE_EVIDENCE_REQUIRED_001` until all
material axes have retained evidence. Admitted profiles return
`native_profile_admitted` and still keep `accepted_without_profile=false`.

Useful commands:

```bash
ncc machine target-admission doctor --json
ncc machine target-admission families --json
ncc machine target-admission admit riscv32imac-unknown-elf --json
ncc machine target-admission admit i386-unknown-linux-gnu --json
ncc machine target-admission avr-atmega328p-none-elf --json
ncc machine target-admission admit avr-atmega328p-none-elf --json
```

This keeps the language powerful without making a false claim. The programmer
may target these worlds, but `machine` only admits them when the material chain
has real bytes, object output and retained smoke evidence.
