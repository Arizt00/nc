# Machine ASM Absolute Complete

`ncc machine asm-complete absolute --json` is the source of truth for the
declared absolute Machine ASM contract.

The contract is closed for declared nC machine profiles:

- x86_64 prefixes, rare addressing forms including segment overrides,
  RIP-relative memory, `[rel symbol]` memory, rel32-symbol `lea`, explicit
  `moffs8/16/32/64`, and base-less SIB index addressing, register classes,
  legacy SSE/SSE2/SSE4.1 SIMD rows, VEX, AVX2, EVEX and AVX-512
  subfeatures. Legacy SIMD includes xmm load/store, packed/scalar FP,
  scalar FP integer conversions and comparisons, packed integer
  add/sub/mul/test, packed shifts, `pshufd`, `shufps` and `blendps` over
  reg-reg, reg-mem and immediate forms where the ISA defines them. Atomic
  bytes include sized `xadd/cmpxchg`, `cmpxchg8b`, `cmpxchg16b` and lockable
  memory RMW forms for the admitted profile.
- AArch64 deep profile coverage, with NEON native encoder forms and the
  admitted SVE profile closed for predicate setup, `whilelo`, unit-stride
  `ld1/st1` b/h/w/d, `cnt/inc` b/h/w/d, predicated FP arithmetic,
  integer add/sub, SVE2 integer multiply, logical vectors, predicate logic,
  compare/select and `faddv`; scalar FP arithmetic, unary ops, min/max,
  compare, conditional select and int/fp conversions, exclusive atomics,
  acquire/release `ldar/stlr`, LSE `cas/swp` and `ldadd/ldclr/ldeor/ldset`
  variants, and unscaled `ldur/stur` are also native. SME and
  unadmitted SVE extension families stay behind explicit catalog gates.
- RISC-V deep profile coverage, with vector, crypto and privileged families
  behind explicit catalog gates until admitted as native profile forms. The
  admitted scalar atomic lane emits AMO `.w`/`.d` bytes, including
  `.aq/.rl/.aqrl` ordering suffixes where the target profile permits them.
- NASM, YASM, GAS Intel, GAS AT&T, MASM, FASM and LLVM MC dialect profiles via
  explicit import/export loss models.

Acceptance rule:

1. A form accepted by `.nm` must parse.
2. It must validate through sema/profile gates.
3. It must lower into machine audit/IR evidence.
4. It must emit bytes/object evidence or be rejected with a typed diagnostic.

The contract does not claim every vendor-private opcode or every historical
assembler quirk. Those are external universe entries until a profile admits
them.
