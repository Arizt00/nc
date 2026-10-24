# Machine ASM-Complete Scope

`[@machine]` must eventually cover:

| Area | Required for ASM parity | Current status |
|---|---|---|
| x86_64 declared native profile | Required for Level 7 | closed for declared profile |
| x86_64 arbitrary whole-ISA universe | Outside declared release profile | not claimed |
| x86_64 addressing modes in declared profile | Required for Level 7 | closed for declared profile |
| REX/VEX/EVEX in declared profile | Required for Level 7 | closed for declared profile |
| ELF/COFF/Mach-O/WASM object emission | Required | closed |
| Relocations in declared object/link profiles | Required | closed for declared profile |
| Debug/unwind | Required for Level 7+ | closed |
| Macros and conditional assembly | Required for Level 10 | closed |
| Disassembly and roundtrip | Required for Level 7+ | closed for declared profile |
| ASM import/export | Required for Level 9 | closed for declared profile; arbitrary lossless historical parity not claimed |
| AArch64/RISC-V/WASM declared profile coverage | Required for Level 8 | closed |

The active commands are:

- `ncc machine asm-complete doctor --json`
- `ncc machine instruction-coverage x86_64 --json`
- `ncc machine encoding-coverage x86_64 --json`
- `ncc machine features x86_64 --json`
- `ncc machine cpu-profile <profile> --json`

These commands are release gates because they make profile boundaries visible
instead of hiding them.

External authority commands are also part of the scope:

- `ncc machine asm-complete toolchain --json`
- `ncc machine asm-complete assemble <file.nm> --tool <llvm-mc|clang|nasm|gas> --target <triple> --out <object>`
- `ncc machine asm-complete disasm-object <object> --tool <llvm-objdump|objdump>`
- `ncc machine asm-complete roundtrip-authority <file.nm> --tool llvm-mc --target <triple>`
- `ncc machine asm-complete parity <file.nm> --target <triple>`

These commands let nC use industrial assembler authorities while preserving `.nm`
as the official language contract.
