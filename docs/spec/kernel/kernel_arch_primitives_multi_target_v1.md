# Kernel architecture primitives v1

## Source contract

Kernel code remains normal `.n` and needs no kernel import or `[@kernel]`:

```text
ncc build kernel.n --profile kernel --target <triple>
```

The compact names coexist with nC naming:

```text
arch_capability(id)
cpu_relax()
memory_barrier()
instruction_barrier()
interrupt_enable()
interrupt_disable()
cpu_halt()
cycle_counter()
memcpy(destination, source, bytes)
memset(destination, value, bytes)
```

Capability identifiers `1..9` correspond to relax, memory barrier, instruction
barrier, interrupt enable, interrupt disable, halt, cycle counter, atomic32 and
MMIO. Physical targets report these capabilities. A virtual target without
physical interrupt, atomic or MMIO state reports `false`; it does not emulate
privileged hardware and claim success.

## Material lowering

- x86/i386: `pause`, `mfence`, `lfence`, `sti`, `cli`, `hlt` and the LLVM cycle
  counter intrinsic.
- AArch64: `yield`, `dmb ish`, `isb`, DAIF enable/disable, `wfi` and the cycle
  counter intrinsic.
- RV32/RV64: architectural encodings for pause, fences, S-status interrupt
  changes and `wfi`, plus the cycle counter intrinsic.
- wasm32: explicit unsupported capability results for physical operations.

The machine encoder also owns concrete bytes for AArch64 `yield`/`wfi` and
RISC-V `pause`/`wfi`. Kernel LLVM, the retained machine layer and byte tests are
therefore aligned.

These operations are privileged. They are emitted only by a kernel profile and
must not be called from a hosted user process.

## Memory compatibility

`memcpy` and `memset` use byte counts and return the destination pointer, as in
C. Their bodies are retained in kernel LLVM and use volatile byte access where
the kernel contract requires observable memory operations.

`memmove` preserves overlapping ranges and `memcmp` returns the first unsigned
byte difference. The low-level C surface also exposes typed `atomic32_*`,
`spin_*`, `mmio_read*`, `mmio_write*`, `read_barrier` and `write_barrier`
intrinsics. Their Machine dependency is the admitted atomic and memory ISA of
each physical target; wasm keeps an explicit unsupported atomic contract.
