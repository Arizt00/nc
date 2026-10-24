# Machine Conformance

| Level | Requirement | Current |
|---|---|---|
| Level 0 | file classification, parser, diagnostics, `fn:`/`fa:` forbidden | closed |
| Level 1 | registers, operands, instruction subset, sema | closed |
| Level 2 | MachineIR, verifier, stack/frame, ABI checks | closed |
| Level 3 | x86_64 encoding subset, bytes, labels, branches | closed |
| Level 4 | ELF/COFF/Mach-O/WASM objects, sections, symbols, relocations | closed for x86_64 plus target artifacts for aarch64/riscv64/wasm32 |
| Level 5 | link/executable smoke, multi-host CI, external symbols | implementation and workflow contract closed; updated retained exact-head artifacts pending |
| Level 6 | full target family closure and basal-ready final closure | implementation closed for x86_64, aarch64, riscv64, wasm32, COFF, Mach-O, and WASM; updated aggregate CI artifact pending |
| Level 7 | x86_64 ASM profile, operand/addressing forms, debug/unwind, disasm and roundtrip | closed for declared profile; arbitrary whole-ISA universe not claimed |
| Level 8 | multi-ISA declared profiles for x86_64, AArch64, RISC-V, and WASM | closed for declared profiles; arbitrary ISA expansion not claimed |
| Level 9 | industrial assembler authority/parity profiles, retained parity evidence, debug info, linker integration, ASM import/export gates | closed for declared profiles; unconditional lossless historical dialect parity not claimed |

Basal can use Level 6 for object/binary closure. Broader assembler-universe
claims must name their declared profile instead of relying on a blanket ASM
claim.

`[@machine]` remains contractual at every level: `fn:` and `fa:` remain forbidden, unsupported instructions fail or report explicit deferred status, and object emission still requires verification.
