# Machine Known Limitations

Native ASM total is closed for declared profiles. The current source of truth is
[Machine Source Of Truth](machine_source_of_truth.md).

Closed-for-profile does not mean every possible external assembler, platform,
ISA extension, binary format, debug format, or vendor behavior is complete.

Not claimed:

- arbitrary whole-universe x86_64/AArch64/RISC-V/WASM ISA coverage outside the
  declared nC profiles;
- GPU, DSP, firmware and legacy microcontroller targets without fully evidenced
  admitted profiles. `riscv32imac-unknown-elf` and `i386-unknown-linux-gnu` are
  now admitted ELF32 native CPU profiles; `armv7em-none-eabi` and the remaining
  non-hosted families stay blocked until bytes, object/image output,
  relocations, loader smoke and corpus evidence exist;
- unconditional lossless parity for every NASM, GAS, MASM, FASM, YASM and
  LLVM-MC historical dialect feature;
- import-library and platform SDK behavior outside the declared linker profiles;
- exact-head multi-host CI success until GitHub Actions uploads the retained row
  and aggregate artifacts for the updated workflow.

Closed admission-gate expansion areas:

- ARM32/Thumb, MIPS32 and PowerPC32. RV32IMAC and i386 are now admitted as
  explicit ELF32 profiles, not as whole-universe ISA promises.
- AVR, MSP430, 8051, 6502, Z80, 68000 and PIC.
- GPU and DSP ISAs.
- Firmware/boot/ROM/flat-binary images.

These areas are not native-target promises. They require target triples,
ABI/data-layout contracts, object or image writers, relocation policies,
hardware/emulator smoke evidence and corpus coverage before admission.

Optional future expansion areas outside this gate:

- Advanced PDB/CodeView.
