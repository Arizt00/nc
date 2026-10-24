# Machine ASM Import Export

`.nm` remains official. ASM import/export is tooling.

Current status:

- `ncc machine export-asm <file.nm> --syntax nasm|gas-intel|gas-att --out <file>`
- `ncc machine import-asm <file.asm> --syntax nasm|gas-intel|gas-att --out <file.nm>`
- basic instruction import/export is available for the verified subset.
- assembler directives that cannot be mapped are rejected with `E_MACHINE_ASM_DIRECTIVE_UNSUPPORTED_001`.

Lossless NASM/GAS/LLVM-MC-class parity is deferred.
