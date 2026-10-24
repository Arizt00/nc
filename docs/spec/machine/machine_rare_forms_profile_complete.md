# Machine Rare Forms Profile Complete

Status: closed for the admitted rare-form profile.

This closure covers rare addressing and prefix forms that are accepted by the
current x86_64 machine profile. It does not admit every historical long-mode
quirk or every assembler-specific spelling.

## Closed Forms

- Segment-qualified memory operands with `cs:`, `ds:`, `es:`, `fs:`, `gs:` and
  `ss:` across ModRM/SIB, direct memory, RIP-relative symbolic memory and
  moffs.
- Assembler-style width hints with `ptr`, for example
  `qword ptr fs:[rbx+8]`.
- Base-less SIB absolute disp32 memory, for example `[0x12345678]`,
  `[symbol]` and `[symbol+disp32]`.
- Symbol-relative direct memory, `[symbol]` and `[symbol +/- addend]`, emits
  base-less SIB disp32 bytes and `Abs32` relocation records for `mov` and
  `lea`.
- GPR64 base memory across `rax..r15`, signed disp8/disp32 displacements,
  and load/store materialization through ModRM/SIB bytes.
- Base+disp boundary selection: `disp8` for `[-128, 127]`, `disp32` for the
  rest of signed `i32`, with overflow rejected before emission.
- Base+index SIB encoding with implicit scale 1, REX.X/REX.B coverage for high
  registers, and rejection of `rsp/r12` as indexes.
- Base+index*scale SIB encoding with explicit scales `1/2/4/8`, high-register
  REX.X/REX.B coverage, reversed index/base spelling, and duplicate-index
  rejection.
- Base+index*scale+disp SIB encoding with signed disp8/disp32 boundary
  selection, accumulated displacement terms, high-register REX.X/REX.B,
  segments and overflow rejection.
- Long-mode 32-bit address-size override forms for the admitted profile:
  implicit 32-bit address registers, explicit `addr32`/`a32`, segment-qualified
  addr32 memory, direct symbolic `Abs32`, VSIB addr32 memory and addr32 moffs.
  These emit prefix `0x67` and reject extended `r8d..r15d`, mixed 32/64 address
  registers, and RIP/`rel` spellings under `addr32`.
- Stack-relative `rsp` forms: `[rsp]`, signed disp8/disp32 variants,
  `lea` address materialization, segment-qualified memory and indexed stack
  slots such as `[rsp+rsi*2+disp]`.
- Frame-relative `rbp` forms: `[rbp]`, forced zero `disp8`, signed
  disp8/disp32 variants, `lea` address materialization, segment-qualified
  frame memory and indexed frame slots such as `[rbp+rsi*2+disp]`.
- SIB-required bases such as `[rsp]` and `[r12]`.
- Displacement-required bases such as `[rbp]` and `[r13]`.
- RIP-relative displacement memory, `[rip+disp32]` and `[rip-disp32]`.
- Symbolic RIP-relative memory, `[rel symbol +/- addend]`,
  `[rip+symbol +/- addend]` and `[symbol+rip +/- addend]`, emits `PcRel32`
  relocation records.
- Symbolic direct memory emits `Abs32` relocation records; symbolic
  RIP-relative memory remains separate and never reuses direct-memory `Abs32`.
- Section-relative memory accepts explicit `section(.name)` terms, for example
  `[section(.data)+disp32]` and `[rel section(.text)+disp32]`. Direct forms
  emit `Abs32`; `rel` forms emit `PcRel32`.
- Far control transfers are closed for the admitted real/compatibility byte
  profile: `ljmp/lcall` immediate `ptr16:16` and `ptr16:32`, indirect far
  memory through `FF /5` and `FF /3`, generic `jmp/call far`, width aliases
  `jmpf16/32` and `callf16/32`, `retf`, `retfw`, `retfd`, `retf imm16`,
  `retfq`, `iret`, `iretw`, `iretd` and `iretq`.
- Vendor-private and assembler-only pseudo spellings outside the declared
  profile fail with typed diagnostics instead of being treated as unknown text
  or deferred bytes.
  This includes representative GAS, NASM/YASM and MASM-style pseudo-ops such
  as `.byte`, `.section`, `.globl`, `.incbin`, `.cfi_startproc`,
  `%macro`, `times`, `dup`, `org`, `bits`, `equ`, `proc`, `invoke`,
  `format`, `entry`, `offset`, `seg` and `assume`.
- Explicit moffs operands:

```text
mov al, moffs8 [0x1122334455667788]
mov moffs8 [0x1122334455667788], al
mov ax, moffs16 [0x1122334455667788]
mov moffs16 [0x1122334455667788], ax
mov eax, moffs32 [0x1122334455667788]
mov moffs32 [0x1122334455667788], eax
mov rax, moffs64 [0x1122334455667788]
mov moffs64 [0x1122334455667788], rax
```

  These are accumulator-only absolute-address opcodes, not ModRM/SIB memory.

## Evidence

- `machine_encoder_tests::x86_64_scalar_extended_encoder_bytes`.
- `machine_encoder_tests::x86_64_base_addressing_encoder_matrix`.
- `machine_encoder_tests::x86_64_base_disp_boundaries_encoder_matrix`.
- `machine_encoder_tests::x86_64_base_index_encoder_matrix`.
- `machine_encoder_tests::x86_64_base_index_scale_encoder_matrix`.
- `machine_encoder_tests::x86_64_base_index_scale_disp_encoder_matrix`.
- `machine_encoder_tests::x86_64_rel_memory_mov_emits_pcrel_relocation`.
- `machine_encoder_tests::x86_64_rip_relative_encoder_and_relocation_matrix`.
- `machine_encoder_tests::x86_64_absolute_moffs_encoder_matrix`.
- `machine_encoder_tests::x86_64_segment_override_encoder_matrix`.
- `machine_encoder_tests::x86_64_stack_relative_encoder_matrix`.
- `machine_encoder_tests::x86_64_frame_relative_encoder_matrix`.
- `machine_encoder_tests::x86_64_symbol_relative_encoder_and_relocation_matrix`.
- `machine_encoder_tests::x86_64_address_size_override_encoder_matrix`.
- `machine_encoder_tests::x86_64_section_relative_encoder_and_relocation_matrix`.
- `machine_encoder_tests::x86_far_control_transfer_encoder_matrix`.
- `machine_encoder_tests::x86_64_manifest_boundary_rejects_vendor_obsolete_and_not_admitted`.
- `machine_encoder_tests::x86_64_assembler_only_pseudo_spellings_are_typed_rejections`.
- `cargo test -p ncc-cli machine_encoder_tests --lib -- --nocapture`.
- `cargo check -p ncc-cli`.
