# Machine Link Relocations

The linker applies final relocation records for declared target profiles and reports every application. Unsupported relocation kinds, target mismatches, overflow, forbidden dynamic relocations, and PC-relative range failures are diagnostics.

Covered profile families include ELF64 x86_64, ELF64 AArch64, ELF64 RISC-V, COFF AMD64, Mach-O x86_64, and WASM function/export fixups.

ELF64 final executable relocation coverage is architectural, not only generic:

- x86_64 applies absolute, signed/unsigned 32-bit, PC-relative, PLT32, and GOTPCREL/GOTPCRELX forms.
- AArch64 applies branch26, page-relative ADRP, low-12 ADD/LDST, ABS32/ABS64, and GOT page/low-12 pairs.
- RISC-V applies branch, JAL, AUIPC/JALR call pairs, absolute 32/64, HI20/LO12, PCREL HI/LO, GOT_HI20 with paired PCREL_LO12, and skips linker-relaxation markers explicitly.
- `.data` relocation records are applied after final layout for absolute and supported PC-relative forms.

GOT-bearing static ELF relocations allocate deterministic final GOT slots before `.text`, write the resolved target VA into each slot, and patch instruction relocations against that slot. Overflow, missing pair labels, unsupported sections, and range failures remain hard diagnostics.
