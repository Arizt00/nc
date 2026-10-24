# Machine ASM-Complete

`[@machine]` is not raw ASM text. It is ASM-level capability expressed through nC contracts: target, ABI, sections, symbols, proc/routine/block, instr, verify, emit, and seal.

The current release state is:

- Machine Level 6 remains closed for target-family binary closure.
- `asm_complete_total` is true for declared native machine profiles after the
  Phase 9 audit and release seal.
- `.nm` classification remains active.
- ELF, COFF, Mach-O, and WASM object emission remain available.
- Updated multi-host retained CI artifacts are still required before claiming
  the latest exact-head external workflow run.

ASM-complete reporting remains active so any future instruction family, prefix,
feature, object metadata area, macro feature, and roundtrip path outside the
declared profile stays visible instead of becoming an implicit claim.

No positive `[@machine]` example may contain `fn:` or `fa:`.

External authority integration is active:

- LLVM MC can assemble derived `.nm` contracts.
- Clang integrated assembler can assemble derived contracts.
- NASM can assemble x86_64 derived contracts.
- GNU `as` can assemble host x86_64 and cross AArch64/RISC-V contracts when installed.
- `llvm-objdump`/`objdump` provide object disassembly evidence.

This is a release-grade authority wrapper for declared profiles, not a claim
that every possible external assembler dialect feature is losslessly modeled.

Native assembler phase 1 is now active:

- x86_64 GPR encodings use native REX/ModRM/SIB emission.
- native `setcc` and `cmovcc` are covered for register forms.
- native memory phase 1 covers base, displacement and base+index*scale+disp forms.
- native vector phase 2 covers SSE/SSE2 XMM register forms and AVX VEX3 register forms behind explicit feature gates.
- exact-byte tests retain the current scope.

Later phases closed the declared native profile total. The remaining non-claim is
the arbitrary external universe: every ISA form, every historical dialect quirk,
and every vendor/tool behavior outside the declared nC machine profiles.
