# Machine Operands

Operand categories: register, immediate, memory, label, symbol, relocation, section-relative, RIP-relative, stack-relative, and frame-relative.

The checker validates register existence, register width compatibility, immediate range, label/symbol resolution, and userland system-register gates.

Immediate literals are accepted in decimal, hexadecimal `0x`, binary `0b`,
octal `0o`, underscore-grouped, signed, GAS-style `$imm`, and AArch64/RISC-V
`#imm` forms. Encoding must never silently truncate: each immediate is checked
against the storage width or instruction-specific signed/unsigned policy before
bytes are emitted.

x86_64 direct memory accepts numeric `[disp32]` and symbolic `[symbol]` /
`[symbol +/- disp32]`. Numeric direct memory emits base-less SIB disp32 bytes;
symbolic direct memory emits the same byte shape plus an `Abs32` relocation.

x86_64 symbol-relative memory is the symbolic subset of direct memory:
`[symbol]`, `[symbol + addend]` and `[symbol - addend]`. It emits base-less
SIB disp32 bytes, records `Abs32`, accepts segment prefixes for memory
instructions, rejects register terms and duplicate symbols, and never becomes
`PcRel32` unless `rel` or `rip` is written explicitly.

x86_64 section-relative memory uses explicit `section(.name)` operands.
`[section(.data)+8]` records an `Abs32` relocation against `.data`;
`[rel section(.text)+8]` records a `PcRel32` relocation against `.text`.
Section-relative operands are not inferred from arbitrary dotted symbols.

x86_64 base memory accepts all admitted 64-bit general registers as address
bases: `[rax]` through `[r15]`, plus signed disp8/disp32 forms such as
`[rbx-1]` and `[r9+0x12345678]`. `rsp/r12` use SIB encoding; `rbp/r13` use
the required disp8-zero form when no displacement is written.

x86_64 base+disp selection is deterministic: `[-128, 127]` emits signed
`disp8`, every other signed `i32` emits `disp32`, and displacement overflow is
rejected. Multiple displacement terms are accumulated before selecting the
encoding width.

x86_64 base+index accepts `[base + index]` with implicit scale `1`. The first
register is encoded as the SIB base and the second as the SIB index. `rsp` and
`r12` are rejected as indexes because that index field encodes "no index".

x86_64 base+index*scale accepts explicit scales `1`, `2`, `4` and `8` only.
The index may appear before or after the base. Duplicate index terms, 32-bit
index registers and invalid scales are rejected before bytes are emitted.

x86_64 base+index*scale+disp combines the same SIB scale rules with signed
displacement selection. Multiple displacement terms are accumulated, `disp8`
is used for `[-128, 127]`, `disp32` for the rest of signed `i32`, and overflow
is rejected.

x86_64 long-mode address-size override accepts implicit 32-bit address
registers such as `[eax]`, `[esp]`, `[ebp]`, `[eax+ecx*4+disp]`, plus explicit
`addr32[...]` / `a32[...]` markers. It emits prefix `0x67` before REX/VEX/EVEX
and records symbolic direct memory as `Abs32`. `addr32` never means RIP-relative
and cannot mix 32-bit and 64-bit address registers.

x86_64 stack-relative memory is the `rsp`-based subset:
`[rsp]`, `[rsp +/- disp8]`, `[rsp +/- disp32]` and
`[rsp + index*scale +/- disp]`. It emits mandatory SIB bytes, accepts segment
prefixes for memory instructions, rejects `rsp` as an index, and is separate
from frame-relative `rbp`.

x86_64 frame-relative memory is the `rbp`-based subset:
`[rbp]`, `[rbp +/- disp8]`, `[rbp +/- disp32]` and
`[rbp + index*scale +/- disp]`. The plain `[rbp]` form emits a forced zero
`disp8` because ModRM `mod=00 rm=101` is not an `rbp` base in long mode.

x86_64 RIP-relative accepts numeric `[rip +/- disp32]` and symbolic forms
`[rel symbol +/- addend]`, `[rip + symbol +/- addend]` and
`[symbol + rip +/- addend]`. Symbolic forms emit `PcRel32` with the user addend
adjusted by `-4` for the x86 rel32 field.

x86_64 absolute moffs uses explicit `moffs8/16/32/64 [addr]` operands and the
accumulator-only opcodes `A0/A1/A2/A3`. The accumulator width must match the
moffs width, the address must fit unsigned 64-bit, and segment prefixes are
accepted. `moffs* addr32[addr]` emits `0x67` and stores a 32-bit address field.

x86_64 segment override accepts `es:`, `cs:`, `ss:`, `ds:`, `fs:` and `gs:`
before bracketed memory. Segment overrides apply to ModRM/SIB memory,
RIP-relative symbolic memory, direct symbolic memory and moffs. `lea` rejects
segment-qualified memory because it computes addresses rather than reading
through a segment.

x86 far control operands accept explicit far immediate pointers
`ptr16:16 selector:offset` and `ptr16:32 selector:offset`, indirect far memory
such as `fword ptr [rax]`, `tword ptr [rax]` and `far [rax]`, plus
`retf`/`retfw`/`retfd`/`retfq` and `iret`/`iretw`/`iretd`/`iretq`.
Near `call`/`jmp` remain rel32 unless a far operand is spelled.

Diagnostics include `E_MACHINE_REGISTER_UNKNOWN_001`, `E_MACHINE_REGISTER_WIDTH_001`, `E_MACHINE_IMMEDIATE_RANGE_001`, `E_MACHINE_LABEL_UNRESOLVED_001`, and `E_MACHINE_SYSTEM_REGISTER_FORBIDDEN_001`.
