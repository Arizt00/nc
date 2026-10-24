# Machine x86_64 Full ISA

The full x86_64 ISA closure target includes data movement, arithmetic, logic, shifts, control flow, setcc, cmovcc, string/rep forms, stack/frame, x87, MMX, SSE, AVX, AVX2, AVX-512, BMI, crypto, atomic/lock, system, virtualization, memory extension, and CET/security families.

The current declared nC machine profile is closed through the absolute ASM
coverage gate. Accepted source forms must parse, validate, lower through the
machine audit model, and either emit native bytes/object evidence or fail with a
typed diagnostic before they can be part of the profile.

Closed profile axes:

- register catalog: GPR aliases, high-byte aliases, `xmm0..31`, `ymm0..31`,
  `zmm0..31`, `k0..7`, `tmm0..7`, and gated system registers.
- prefixes: lock, REP families, segment overrides, operand/address-size,
  REX, VEX2, VEX3 and EVEX.
- addressing: base, displacement, SIB, RIP-relative, section-relative,
  symbolic/relocatable, segment-qualified and VSIB gather/scatter forms.
- far control: admitted real/compatibility byte forms for far immediate,
  generic `jmp/call far`, indirect far memory, width-specific aliases, far
  return and interrupt return.
- SIMD/vector: SSE family, AVX, FMA/F16C, AVX2, EVEX and the declared AVX-512
  subfeatures.
- catalog-gated families: privileged/system, virtualization, CET/security,
  AMX tile and vendor-private opcode space.

The CLI source of truth is:

```bash
ncc machine asm-complete absolute --json
```

External opcodes outside a declared profile are not silently accepted. They must
be added to the nC profile or a dialect profile before code can depend on them.

## Outside-manifest rejection gates

The declared x86_64 profile rejects four non-profile classes with typed
diagnostics before byte emission:

- vendor-private or vendor-specific instructions outside the profile emit
  `E_MACHINE_VENDOR_PRIVATE_OUTSIDE_MANIFEST_001`.
- obsolete, legacy or long-mode-excluded instructions emit
  `E_MACHINE_OBSOLETE_OUTSIDE_MANIFEST_001`.
- known external ISA families that have not been admitted emit
  `E_MACHINE_NOT_ADMITTED_OUTSIDE_MANIFEST_001`.
- assembler-only pseudo spellings outside the nC machine source model emit
  `E_MACHINE_ASSEMBLER_PSEUDO_OUTSIDE_MANIFEST_001`.
  Representative gated spellings include GAS data/section/CFI directives,
  NASM/YASM `db/res*/times/%macro/bits/org` forms, MASM
  `proc/endp/public/extrn/assume/offset/invoke` forms and FASM
  `format/entry/rb/rw/rd/rq` forms.

Unknown spelling remains separate and emits `E_MACHINE_INSTR_UNKNOWN_001`.
This prevents the compiler from treating vendor/private, obsolete,
assembler-pseudo and not-yet admitted opcodes as generic unknown text or as
deferred native bytes.
