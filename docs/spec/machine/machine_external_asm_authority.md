# Machine External ASM Authority

`.nm` remains the official nC machine source. External assemblers are authority tools used to prove encoding, object materialization, disassembly, and cross-tool parity.

Authority tools:

- LLVM MC through `llvm-mc`.
- LLVM disassembly through `llvm-objdump`.
- Clang integrated assembler.
- NASM for x86_64 Intel syntax.
- GNU `as` for host x86_64.
- GNU cross binutils for AArch64 and RISC-V.
- GNU `objdump` as fallback disassembler.

Commands:

- `ncc machine asm-complete toolchain --json`
- `ncc machine asm-complete assemble <file.nm> --tool llvm-mc --target <triple> --out <object>`
- `ncc machine asm-complete disasm-object <object> --tool llvm-objdump`
- `ncc machine asm-complete roundtrip-authority <file.nm> --tool llvm-mc --target <triple>`
- `ncc machine asm-complete parity <file.nm> --target <triple>`

Rules:

- External ASM is not official source.
- External tool failure is surfaced as evidence.
- Missing tools produce blocked reports, not success.
- `fn:` and `fa:` stay forbidden in machine positives.
- The generated authority assembly is derived from verified MachineIR-like `.nm` structure.
- Object hashes and disassembly output hashes are retained in reports.

This closes the external authority integration layer. It does not replace the later project of writing complete native nC assemblers.
