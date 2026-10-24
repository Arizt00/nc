# nC raw ABI and binary layout v1

`nC.raw.abi.v1` is the material ABI tag carried from high-level IR into machine
plans. Foreign C/OS calls use explicit target bridges; they do not silently
change an nC function's contract.

## Data models

| Target | Model | Pointer/function pointer | Stack alignment | Direct aggregate limit |
|---|---:|---:|---:|---:|
| x86_64 SysV | LP64 | 64 bits | 16 | 16 bytes |
| x86_64 Win64 | LP64 | 64 bits | 16 | 8 bytes |
| i386 cdecl | ILP32 | 32 bits | 4 | 8 bytes |
| AArch64 AAPCS64 | LP64 | 64 bits | 16 | 16 bytes |
| ARMv7 hard-float | ILP32 | 32 bits | 16 | 8 bytes |
| RV32 | ILP32 | 32 bits | 16 | 8 bytes |
| RV64 | LP64 | 64 bits | 16 | 16 bytes |

All listed triples are little-endian. Function pointers have the same width as
data pointers. Large arguments are indirect and their storage is caller-owned.
Large returns use caller-owned `sret`; the hidden location is reserved before
user arguments, including stack-only i386. SysV64 varargs materialize the exact
XMM count in `AL`. Win64 reserves 32 bytes of shadow space, uses positional
GPR/XMM slots and mirrors variadic floating arguments into both register banks.

TLS and per-CPU bases are material registers: `FS.base/GS.base` on x86_64,
`GS/FS` on i386, `TPIDR_EL0/TPIDR_EL1` on AArch64,
`TPIDRURW/TPIDRPRW` on ARMv7 and `tp/sscratch` on RV32/RV64. No lookup or
allocation is injected when either storage class is addressed.

## Layout

`repr(C)`, `packed`, `align(N)`, little/big endian metadata, unions, bitfields
and flexible trailing arrays are computed by one binary-layout authority. TLS
and per-CPU addresses are explicit storage classes. No aggregate assignment is
allowed to invent a byte copy; use `cp` or `mv` with explicit addresses and
`sizeOf<T>()`.

The executable certification is `ncc_machine::certify_all_nc_abi_layouts()`.
It covers the six requested architecture families plus the explicit Win64
variant and verifies sret/register non-aliasing. The 16-byte nC call boundary
is retained on ARMv7 even though an external AAPCS32 bridge may accept a weaker
boundary; bridges perform that adaptation explicitly.
