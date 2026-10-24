# Machine Relocations

Supported relocation contracts:

| Format | Relocations |
|---|---|
| ELF64 x86_64 | `R_X86_64_64`, `R_X86_64_PC32`, `R_X86_64_PLT32`, `R_X86_64_GOTPCREL`, `R_X86_64_GOTPCRELX`, `R_X86_64_REX_GOTPCRELX`, `R_X86_64_32`, `R_X86_64_32S` |
| ELF64 AArch64 | `R_AARCH64_ABS64`, `R_AARCH64_ABS32`, `R_AARCH64_ADR_PREL_PG_HI21`, `R_AARCH64_ADD_ABS_LO12_NC`, `R_AARCH64_LDST64_ABS_LO12_NC`, `R_AARCH64_JUMP26`, `R_AARCH64_CALL26`, `R_AARCH64_ADR_GOT_PAGE`, `R_AARCH64_LD64_GOT_LO12_NC` |
| ELF64 RISC-V | `R_RISCV_64`, `R_RISCV_32`, `R_RISCV_BRANCH`, `R_RISCV_JAL`, `R_RISCV_CALL`, `R_RISCV_CALL_PLT`, `R_RISCV_GOT_HI20`, `R_RISCV_PCREL_HI20`, `R_RISCV_PCREL_LO12_I`, `R_RISCV_PCREL_LO12_S`, `R_RISCV_HI20`, `R_RISCV_LO12_I`, `R_RISCV_LO12_S`, `R_RISCV_RELAX` |
| ELF32 i386 | `R_386_32`, `R_386_PC32`, `R_386_PLT32`, `R_386_GOT32` |
| ELF32 RV32 | `R_RISCV_32`, `R_RISCV_BRANCH`, `R_RISCV_JAL`, `R_RISCV_CALL`, `R_RISCV_CALL_PLT`, `R_RISCV_GOT_HI20`, `R_RISCV_PCREL_HI20`, `R_RISCV_PCREL_LO12_I`, `R_RISCV_PCREL_LO12_S`, `R_RISCV_HI20`, `R_RISCV_LO12_I`, `R_RISCV_LO12_S`, `R_RISCV_RELAX` |
| COFF x86_64 | `IMAGE_REL_AMD64_ADDR64`, `IMAGE_REL_AMD64_REL32` |
| Mach-O x86_64 | `X86_64_RELOC_UNSIGNED`, `X86_64_RELOC_BRANCH`, `X86_64_RELOC_SIGNED` |

`lea r64, [rel symbol]`, `lea r64, qword [rel symbol]`,
`lea r64, [rip + symbol]` and addend-bearing forms such as
`[rel symbol + 8]`, `[rip + symbol - 4]` and `[symbol + rip + 16]` encode as
x86_64 `REX.W 8D /r disp32` for every GPR64 destination (`rax`..`r15`) and
emit a `pc32` relocation against `symbol`. The backend relocation addend is the
source addend minus 4, matching x86 rel32's end-of-field base.

Final executable lowering applies text and data relocations after layout. GOT-using static ELF relocations allocate deterministic GOT slots and patch the instruction-side relocation to the slot address; dynamic imports keep their explicit PLT/GOT metadata lane.

Unsupported relocation types fail with `E_MACHINE_RELOCATION_UNSUPPORTED_001`.
