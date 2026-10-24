# Machine Non-Hosted ISA Gates

`ncc machine target-family-gate non-hosted` is the source of truth for ISA
families that are outside the current hosted native target set.

Closed families:

- GPU: PTX/SASS, AMDGCN, SPIR-V and Metal-like device code.
- DSP: Hexagon, TI C6x, CEVA and Tensilica-style DSPs.
- Native 32-bit CPUs: i386, ARM32/Thumb, MIPS32, RISC-V32 and PowerPC32.
- Firmware: flat binaries, UEFI/BIOS/ROM/boot-sector and freestanding images.
- Legacy microcontrollers: AVR, MSP430, 8051, 6502, Z80, 68000 and PIC.

Closure rule:

1. These families are not silently accepted as CPU machine targets.
2. A `.nm` target for one of them requires a declared profile.
3. A profile must define registers, ABI/data layout, object or image format,
   relocations, linker/loader behavior, execution evidence and corpus coverage.
4. Until then, the family is represented by
   `E_MACHINE_TARGET_FAMILY_PROFILE_REQUIRED_001`.

This is a closure of the frontier, not a false claim that nC already emits every
GPU, DSP or microcontroller binary format.
