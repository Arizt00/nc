# Machine/linker expansion for operational kernel runtime

## New encoded and linked surface

### RISC-V

- Local branch, JAL, compressed jump and call targets are resolved from instruction
  offsets before encoding.
- External JAL/call keeps a zero immediate only when relocation owns the final value.
- Final relocation families include RV32/RV64 abs32/abs64 where applicable, branch,
  JAL, call, call-plt, GOT_HI20, HI20/LO12, PCREL_HI20/LO12, RVC branch/jump and RELAX.

### i386 PIC

- `R_386_GOTPC`: `GOT + A - P`.
- `R_386_GOTOFF`: `S + A - GOT` with signed 32-bit range validation.
- `R_386_GOT32X` shares the material GOT-slot contract of `GOT32`.
- `_GLOBAL_OFFSET_TABLE_` is linker-defined and never emitted as a dynamic import.
- Freestanding RV32 unresolved imports remain rejected; tests provide a real second
  object instead of claiming a dynamic loader.

### AArch64

- Stack displacement spellings `[sp + n]` and `[sp, #n]` converge on one parser.
- 32/64-bit load/store opcodes and scaled low12 relocations preserve operand width.
- BSS alignment is propagated so LD/ST low12 relocations see the same address that
  the system loader maps.

## Final image layout

`Phase12ObjectUnit.bss_align` is collected from ELF, COFF and Mach-O input sections.
`Phase12LinkedImage` retains the maximum alignment and aligns every input BSS base.
Final writers apply it to:

- PE `VirtualSize` and BSS symbol VA;
- ELF32/64 `.bss`, `sh_addralign`, symbol VA and RW `PT_LOAD.p_memsz`;
- Mach-O `__bss` offset/alignment and `__DATA.vmsize`.

The file size is not inflated with fake BSS payload. The loader reserves the aligned
zero-fill range through virtual/memory size.

## Mach-O corrections

- X86_64 SIGNED/SIGNED_1/SIGNED_2/SIGNED_4 map to final PC-relative relocation.
- `nlist_64.n_sect` is read from byte `+5`.
- One leading ABI underscore is removed for global aliases; local symbols are not
  aliased.
- Local identity includes object, section, name and offset, avoiding AArch64 mapping
  symbol collisions.

## Regression authority

The complete `machine_closure` suite passes 776/776. It includes encoders, objects,
third-party import, final relocations, ELF/PE/Mach-O/Wasm, roundtrip, disassembly,
fuzz corpus and system-loader/QEMU gates.
