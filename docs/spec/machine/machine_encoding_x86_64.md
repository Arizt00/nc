# x86_64 Encoding

Closed encoding forms include:

| Form | Bytes |
|---|---|
| `mov rax, imm64` | `48 b8 imm64` |
| `mov rax, rdi` | `48 89 f8` |
| `mov rax, qword ptr [symbol]` | `48 8b 04 25 disp32` + `Abs32` |
| `lea r10, qword ptr [symbol+0x12345678]` | `4c 8d 14 25 78 56 34 12` + `Abs32` |
| `mov rax, qword ptr [rel symbol]` | `48 8b 05 disp32` + `PcRel32` |
| `mov rax, qword ptr [section(.data)+disp32]` | `48 8b 04 25 disp32` + `Abs32` |
| `mov rax, qword ptr [rel section(.text)+disp32]` | `48 8b 05 disp32` + `PcRel32` |
| `mov rax, qword ptr [eax]` | `67 48 8b 00` |
| `mov rax, qword ptr addr32[symbol]` | `67 48 8b 05 disp32` + `Abs32` |
| `mov rax, moffs64 addr32[addr]` | `67 48 a1 addr32` |
| `mov rax, qword ptr [rsp]` | `48 8b 04 24` |
| `load rax, qword ptr [rdi]` | `48 8b 07` (typed alias of `mov`) |
| `store qword ptr [rdi], rsi` | `48 89 37` (typed alias of `mov`) |
| `mov byte ptr [rdi], dl` | `88 17` |
| `mov byte ptr [r8+rax], 0` | `41 c6 04 00 00` |
| `lea r10, qword ptr [rsp+0x20]` | `4c 8d 54 24 20` |
| `mov rax, qword ptr [rbp]` | `48 8b 45 00` |
| `lea r10, qword ptr [rbp-0x20]` | `4c 8d 55 e0` |
| `add rax, rsi` | `48 01 f0` |
| `add rax, rbx` | `48 01 d8` |
| `push rbp` / `pop rbp` | `55` / `5d` |
| `sub rsp, imm8` / `add rsp, imm8` | `48 83 ec/c4 imm8` |
| conditional jump | `0f 8x rel32` |
| `call` | `e8 rel32` |
| `ljmp ptr16:32 selector:offset` | `ea offset32 selector16` |
| `lcall ptr16:16 selector:offset` | `66 9a offset16 selector16` |
| `jmp/call far selector:offset` | `ea/9a offset32 selector16` |
| `ljmp fword ptr [base]` | `ff /5` |
| `lcall tword ptr [base]` | `48 ff /3` |
| `retf` / `retfw` / `retfq` | `cb` / `66 cb` / `48 cb` |
| `iret` / `iretw` / `iretq` | `cf` / `66 cf` / `48 cf` |
| `ret` | `c3` |

Immediate encoding is checked before emission. `mov r64, imm64` accepts signed
64-bit values and unsigned 64-bit bit-pattern literals such as
`0xffff_ffff_ffff_ffff`; values outside the 64-bit storage width fail with
`E_MACHINE_IMMEDIATE_RANGE_001`. ALU immediates select `imm8` or `imm32`
without silent truncation.

Missing operand forms fail with `E_MACHINE_ENCODING_001`.
`load` and `store` do not add an abstract execution path on x86_64: admitted
forms are encoded by the same ModRM/SIB machinery and emit the same bytes as
their corresponding `mov` form.

Byte memory stores accept register and immediate sources through the same
ModRM/SIB path, including base-plus-index addressing. Immediate-to-memory
forms require an explicit memory width (`byte ptr`, `word ptr`, `dword ptr` or
`qword ptr`) so emission never infers or truncates the stored width silently.
