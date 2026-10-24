# Machine lowering for the C11 memory model

Status: closed for x86_64, AArch64 and RISC-V scalar profiles.

This document binds the high-level memory model to concrete machine behavior.
The semantic contract is defined in
`../language/c11_memory_model.md`; this file records target legalization,
instruction selection and object evidence.

## Shared lowering rules

- Atomic storage keeps the payload width and alignment.
- Relaxed operations omit barriers not required by the ISA.
- Acquire, release, `acq_rel` and `seq_cst` are preserved in IR.
- RMW and compare-exchange remain indivisible through optimization.
- Compiler fences become side-effecting compiler barriers.
- Volatile operations remain volatile in LLVM and object generation.
- Wait loops reread atomically and use a target pause/wait hint.

## x86_64

The backend selects ordinary atomic loads/stores where TSO is sufficient,
locked `xadd` for fetch add/sub, locked compare-exchange loops for bitwise RMW
and `cmpxchg` for compare-exchange. `mfence`/locked ordering and `pause` cover
thread fences and wait loops. Direct machine encoding also covers sized
`xadd`, `cmpxchg`, `cmpxchg8b`, `cmpxchg16b` and admitted `lock` RMW forms.

Both SysV64 and Win64 use the same memory semantics while preserving their
own ABI register and stack rules.

## AArch64

Loads/stores select `ldar` and `stlr` when acquire/release ordering is needed.
RMW and compare-exchange use `ldxr`/`stxr` or `ldaxr`/`stlxr` retry loops;
admitted LSE targets may select `cas`, `swp`, `ldadd`, `ldclr`, `ldeor` and
`ldset` order variants. `dmb` implements required full barriers. Wait and
notification use `sevl`, `wfe` and `sev`.

## RISC-V

RV32 and RV64 use AMO operations with `.aq`, `.rl` or `.aqrl` bits for RMW.
Compare-exchange uses `lr.w`/`sc.w` or `lr.d`/`sc.d` loops. Loads/stores pair
with the fences required by the selected order. Wait loops use the standard
pause hint encoding where the admitted ISA profile supports it.

The final ELF linker preserves local symbol identity by ELF symbol-table
index. Identically named local labels from different input objects therefore
cannot collide during relocation.

## Object and execution evidence

The permanent end-to-end matrix is
`crates/ncc-cli/tests/core_alloc_c11_native_e2e.rs`. It checks:

- direct atomic LLVM operations and successful object compilation;
- x86_64 release build and host execution;
- AArch64 object disassembly containing ordered load/store, exclusive RMW,
  barriers, wait and notification instructions;
- RV64 static ELF disassembly containing fences, AMO and LR/SC operations;
- RV64 execution with QEMU when the runner provides `qemu-riscv64`.

Exact byte matrices also live in `crates/ncc-ir/src/c11_memory.rs` and the
machine encoder tests in `crates/ncc-cli/src/commands/machine.rs`.

The industrial closure manifest emits 315 closed rows across SysV64, Win64,
AArch64, RV32 and RV64. Its host-smoke plan contains 140 Win64/AArch64 rows;
RISC-V evidence is retained by target object/disassembly tests and QEMU
execution when available. Counts are asserted by tests and are not descriptive
documentation-only totals.

Normative commands:

```text
cargo test -p ncc-ir c11_memory -- --nocapture
cargo test -p ncc-cli machine_encoder_tests --lib -- --nocapture
cargo test -p ncc-cli --test core_alloc_c11_native_e2e -- --nocapture
```
