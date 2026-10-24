# Machine Atomic Profile Complete

Status: closed for the admitted machine atomic profile.

The canonical high-level order, type and synchronization contract is defined
in [C11-style memory model](../language/c11_memory_model.md). The complete
multi-target mapping is defined in
[Machine lowering for the C11 memory model](machine_c11_atomic_lowering_multitarget.md).

This profile records the atomic instructions that lower to concrete bytes in
the native machine encoder. It is not a claim that every historical or
vendor-private atomic instruction in every ISA is admitted.

## x86_64

The encoder materializes:

- `xadd` and `cmpxchg` over byte, word, dword and qword register/memory forms;
- `cmpxchg8b` over qword memory;
- `cmpxchg16b` over 128-bit memory;
- `lock` prefix over admitted memory-writing RMW families: `add`, `adc`,
  `and`, `or`, `xor`, `sub`, `sbb`, `inc`, `dec`, `neg`, `not`, `bts`, `btr`,
  `btc`, `xadd`, `cmpxchg`, `cmpxchg8b` and `cmpxchg16b`.

Memory operands accept compact and assembler-style width hints such as
`qword [rax]` and `qword ptr [rax]`. The first operand must be memory for a
`lock`-prefixed instruction to encode.

## AArch64

The encoder materializes:

- exclusive loads/stores: `ldxr`, `stxr`, `ldaxr`, `stlxr`;
- acquire/release ordered loads/stores: `ldar`, `stlr`;
- LSE compare/exchange: `cas`, `casa`, `casl`, `casal`;
- LSE swap: `swp`, `swpa`, `swpl`, `swpal`;
- LSE read-modify-write: `ldadd`, `ldadda`, `ldaddl`, `ldaddal`, `ldclr`,
  `ldclra`, `ldclrl`, `ldclral`, `ldeor`, `ldeora`, `ldeorl`, `ldeoral`,
  `ldset`, `ldseta`, `ldsetl`, `ldsetal`.

The direct machine profile covers 32-bit and 64-bit general-register forms.
The LLVM/MCT high-level path legalizes payload width against the selected
target and preserves the requested memory order.

## RISC-V

The encoder materializes AMO instructions with relaxed, acquire, release and
acquire-release suffixes:

- RV32: `.w` forms for `lr`, `sc`, `amoswap`, `amoadd`, `amoxor`, `amoand`,
  `amoor`, `amomin`, `amomax`, `amominu`, `amomaxu`;
- RV64: the same `.w` forms plus `.d` forms.

Ordering suffixes are parsed as `.aq`, `.rl` and `.aqrl`, for example:

```text
lr.w.aq a0, (a1)
amoadd.w.aqrl a0, a2, (a1)
amoswap.d.rl a0, a2, (a1)
```

RV32 rejects `.d` forms at the profile gate.

## Evidence

The byte-level regression suite is `machine_encoder_tests` in
`crates/ncc-cli/src/commands/machine.rs`. High-level-to-object and execution
evidence lives in `crates/ncc-cli/tests/core_alloc_c11_native_e2e.rs`.

The local verification command for this closure is:

```text
cargo test -p ncc-cli machine_encoder_tests --lib -- --nocapture
cargo test -p ncc-cli --test core_alloc_c11_native_e2e -- --nocapture
```
