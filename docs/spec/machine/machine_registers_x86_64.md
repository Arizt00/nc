# x86_64 Registers

Status: closed for the admitted x86_64 machine register profile.

## Userland register classes

| Class | Registers |
|---|---|
| 64-bit GPR | `rax rbx rcx rdx rsi rdi rbp rsp r8-r15` |
| 32-bit views | `eax ebx ecx edx esi edi ebp esp r8d-r15d` |
| 16-bit views | `ax bx cx dx si di bp sp r8w-r15w` |
| low 8-bit views | `al bl cl dl sil dil bpl spl r8b-r15b` |
| legacy high 8-bit | `ah ch dh bh` |
| instruction pointer | `rip` |
| flags | `ZF CF SF OF PF AF DF IF` |
| MMX | `mm0-mm7` |
| x87 stack | `st0-st7` |
| SIMD | `xmm0-xmm31`, `ymm0-ymm31`, `zmm0-zmm31` |
| AVX-512 masks | `k0-k7` |
| AMX tiles | `tmm0-tmm7` |
| segment selectors / overrides | `cs ds es fs gs ss` |

`machine registers x86_64 --json` exposes the same classes plus ABI roles for
SysV64 and Win64.

## Privileged registers

Control registers `cr0-cr15`, debug registers `dr0-dr15`, descriptor registers
`gdtr idtr ldtr tr`, `xcr0`, `msr` and `efer` are cataloged as system
registers. They are rejected by default in the userland profile with
`E_MACHINE_SYSTEM_REGISTER_FORBIDDEN_001`.

## Encoding rule

Legacy high-byte registers `ah ch dh bh` cannot appear in an instruction that
requires a REX prefix. The verifier emits `E_MACHINE_REGISTER_REX_HIGH8_001`
for that case before byte emission.

## Register instruction encodings

The admitted register-instruction surface has native bytes for:

- `mov` between GPR and control/debug registers.
- `mov` between segment registers and 16-bit GPR/memory operands.
- descriptor/task/system memory forms: `lgdt`, `lidt`, `sgdt`, `sidt`, `ltr`,
  `str`, `invlpg`, `vmxon`, `vmclear`, `vmxoff`, `vmlaunch`, `vmresume`,
  `clts`, `invd`, `wbinvd`, `swapgs`, `sysenter`, `sysexit`, `rsm`,
  `monitor`, `mwait`, `clac`, `stac`, `int`, `int3`, `in`, `out`.
- `xgetbv` and `xsetbv`.
- x87 stack, constant, status/control and memory forms: `fwait`, `fnop`,
  `finit`, `fninit`, `fclex`, `fnclex`, `fld`, `fstp`, `fldcw`, `fstcw`,
  `fnstcw`, `fstsw`, `fnstsw`, `fldenv`, `fstenv`, `fnstenv`, `frstor`,
  `fsave`, `fnsave`, `ffree`, `fxch`, `fcom*`, `fucom*`, `fchs`, `fabs`,
  `ftst`, `fxam`, `fld*`, `f2xm1`, `fyl2x`, `fptan`, `fpatan`, `fxtract`,
  `fprem*`, `fsqrt`, `fsincos`, `frndint`, `fscale`, `fsin`, `fcos`, `flog*`,
  `fild`, `fistp`, `fadd`, `fsub`, `fmul`, `fdiv`, `emms`.
- MMX forms: `movq`, packed integer arithmetic/logic, compares, unpack,
  saturating add/sub, multiply and shifts on `mm0-mm7` and `mmword` memory.
- AMX tile forms: `ldtilecfg`, `sttilecfg`, `tileloadd`, `tileloaddt1`,
  `tilestored`, `tilerelease`, `tilezero`, `tdpbf16ps`, `tdpbssd`,
  `tdpbsud`, `tdpbusd`, `tdpbuud`.

AMX tile load/store memory operands are encoded through SIB memory, matching
the architectural requirement for tile memory forms.
