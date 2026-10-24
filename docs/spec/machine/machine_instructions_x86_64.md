# x86_64 Instruction Subset

Closed initial subset: `mov`, `lea`, `push`, `pop`, `xchg`, `add`, `sub`, `imul`, `idiv`, `mul`, `inc`, `dec`, `neg`, `and`, `or`, `xor`, `not`, `shl`, `shr`, `sar`, `cmp`, `test`, `jmp`, `je`, `jne`, `jl`, `jle`, `jg`, `jge`, `jo`, `jb`, `jbe`, `ja`, `jae`, `call`, `ret`, `leave`, `nop`, and gated `hlt`.

In typed `[@machine]` source, signed-overflow branching is canonicalized as
`jmp.ov label`; the x86 spelling `jo label` is an accepted alias. Sema admits
that branch only while signed-overflow flags from `add`, `sub`, `mul`, `adc`,
`sbb`, `neg.bin`, `inc` or `dec` are live. A `mov`, call, memory operation or
other flag-clobbering boundary invalidates that state. x86-64 emission uses the
near `JO rel32` opcode (`0F 80`).

Privilege-gated instructions include `syscall`, `int`, `cli`, and `sti`.

Unsupported mnemonics fail with `E_MACHINE_INSTR_UNKNOWN_001`.
