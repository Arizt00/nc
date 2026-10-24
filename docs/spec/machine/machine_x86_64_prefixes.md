# Machine x86_64 Prefixes

Required prefix families for ASM-complete:

- lock
- rep and repne
- segment override
- operand-size override
- address-size override
- REX
- VEX
- EVEX

Current closure emits the admitted prefix families as native bytes:

- `lock` over memory-writing atomic/RMW forms;
- `rep`, `repe/repz` and `repne/repnz` over string forms;
- segment override prefixes `cs`, `ds`, `es`, `fs`, `gs` and `ss`;
- operand-size override for admitted word-sized forms;
- address-size override `0x67` for admitted long-mode addr32 memory and moffs
  forms;
- REX `W/R/X/B` for GPR, SIB and extended register forms;
- VEX2/VEX3 and EVEX in the admitted vector profile.

The addr32 profile is explicit: accepted source must use a 32-bit address
register or `addr32`/`a32`, and mixed RIP-relative or 64-bit address-register
forms remain rejected before byte emission.
