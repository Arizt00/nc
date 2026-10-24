# Machine VEX ModRM And SIB

Closed Phase 1C VEX forms use native ModRM and SIB generation.

Rules:

- Register/register forms use ModRM `mod=11`.
- Register/memory and memory/register forms parse the memory operand before encoding.
- Memory operands require an allowed VEX memory width such as `xmmword` or `ymmword`.
- SIB is emitted when the base or index form requires it.
- `rsp` cannot be used as an index register.
- 32-bit base registers such as `eax` are rejected in 64-bit memory addressing.
- Invalid ModRM addressing reports `E_MACHINE_MODRM_001`.
- Invalid SIB addressing reports `E_MACHINE_SIB_001`.

Immediate forms use checked `imm8` emission. Out-of-range immediates report `E_MACHINE_IMM_WIDTH_001`; no silent truncation is allowed.
