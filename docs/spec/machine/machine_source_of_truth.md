# Machine Source Of Truth

This file is the current documentation anchor for `[@machine]` status.
Older phase notes are historical unless they agree with this table and the
current `ncc machine doctor --json` surface.

| Area | Current status |
|---|---|
| Parser, sema, MachineIR, verifier | closed |
| ELF64, COFF, Mach-O and WASM object/module emission | closed |
| Debug/unwind metadata | closed in Phase 5 |
| Object-aware disassembly | closed in Phase 6 |
| Native roundtrip | closed in Phase 7 |
| AArch64, RISC-V64, RISC-V32, i386 and WASM declared profile parity | closed in Phase 8 plus admitted ELF32 profiles |
| Native ASM total for declared profiles | closed after Phase 9 |
| Vendor-private, obsolete, assembler-pseudo and outside-manifest x86_64 spellings | closed as typed rejection gates |
| Macro/directive system | closed in Phase 10 |
| ASM dialect import/export tooling | closed for declared profiles; arbitrary lossless historical dialect parity is not claimed |
| Linker integration and OS-loader executable writers | closed in Phase 12 for declared targets |
| External compatibility validation | closed in Phase 13 for declared profiles |
| Fuzzing/corpus hardening | closed in Phase 14 |
| GPU, DSP, 32-bit CPU, firmware and legacy microcontroller ISA frontier | explicit gates declared; `riscv32imac-unknown-elf` and `i386-unknown-linux-gnu` are admitted ELF32 native profiles; the rest remain blocked until evidence |
| Release seal | closed for declared machine profiles |
| Updated multi-host retained CI run | contract implemented; exact-head retained artifacts pending until GitHub Actions runs |

The phrase "closed" means closed for the declared nC machine profile, not for
every instruction, object format, platform ABI, vendor extension, historical
assembler dialect, or external tool behavior that exists outside the profile.

Still not claimed as native targets:

- arbitrary whole-universe x86_64/AArch64/RISC-V/WASM ISA coverage outside the declared profiles;
- vendor-private, obsolete, known external x86_64 instructions and assembler-only pseudo spellings outside the manifest are rejected with typed diagnostics until admitted through a profile or dialect import;
- GPU, DSP, firmware and legacy microcontroller ISAs without a fully evidenced admitted profile. Representative admission contracts are exposed by `ncc machine target-admission`, and only `riscv32imac-unknown-elf` plus `i386-unknown-linux-gnu` are accepted as 32-bit native CPU targets today;
- unconditional lossless parity with every NASM, GAS, MASM, FASM, YASM and LLVM-MC dialect feature;
- external CI success until the updated retained multi-host workflow has uploaded its row and aggregate artifacts.
