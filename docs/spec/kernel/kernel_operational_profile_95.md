# Kernel operational profile: measured 95 percent

> Historical snapshot. The logical SMP core described as the remaining axis is
> now closed by `kernel_smp_core_contract_v1.md`. Physical AP startup and
> inter-CPU hardware synchronization remain platform work and are not claimed.

## Scope

This percentage describes the declared `nC kernel core` profile. It does not mean
95 percent of every possible desktop/server/embedded operating system. The profile
contains 20 auditable axes; 19 are closed end to end.

## Closed axes (19)

1. Normal `.n` source without mandatory `[@kernel]`.
2. Typed `kmain` entry and per-target boot argument ABI.
3. `ncc_kernel_boot_info_v1` binary layout.
4. Loader memory-map handoff.
5. Serial descriptor/handoff.
6. Optional framebuffer descriptor contract.
7. Typed ACPI/UEFI/Multiboot/E820/FDT/OpenSBI platform slots.
8. Frame allocator init/alloc/free.
9. Page map/unmap/protect/translate.
10. Task register/spawn/block/wake/exit.
11. Scheduler open/next/tick/yield.
12. IRQ register/enable/disable/state/EOI.
13. Syscall register/availability and native handler dispatch ABI.
14. Driver register/start/stop/status.
15. User-to-kernel and kernel-to-user copy.
16. Panic/halt/quarantine observable state.
17. Hosted interpreter state parity for safe observable operations.
18. Full high-level body lowering through LLVM/MCT and machine wrapper.
19. Object/final-link matrix for eleven declared targets plus BIOS QEMU x86_64/i386.

## SMP axis update

Closed after this snapshot: typed CPU registration/state, per-CPU storage, task
affinity, scheduler ownership per CPU, FIFO IPI semantics, hosted execution, LLVM
materialization and the eleven-target object/link matrix.

Still physical-platform work: AP power-on/bring-up through INIT/SIPI, PSCI or SBI,
topology discovery, hardware interrupt-controller delivery, atomic barriers and
multicore hardware certification.

## Target matrix

- `x86_64-unknown-none`
- `x86_64-unknown-linux-gnu`
- `x86_64-pc-windows-msvc`
- `x86_64-apple-darwin`
- `aarch64-unknown-none`
- `aarch64-unknown-linux-gnu`
- `riscv64-unknown-elf`
- `riscv32imac-unknown-elf`
- `i386-unknown-none-elf`
- `i386-unknown-linux-gnu`
- `wasm32-unknown-unknown`

The current operational example is `examples/11_machine/kernel_operational_smp.n`. Every
target produces its retained `.nm`, machine object, lowering manifest and image/link
artifact. Native targets also retain the high-level body LLVM and native object.

## Boot evidence and honest boundary

x86_64 and i386 BIOS images execute stage0/stage1, ACPI RSDP probe, E820 probe, ELF
segment loading and `kmain` in QEMU. Other targets currently prove target-native body,
object, relocation and final link contracts; full physical UEFI/OpenSBI/FDT boot
loaders are a later platform layer and are not mislabeled as executed hardware.

The hosted interpreter records syscall registration but does not jump to an
arbitrary raw handler address inside the compiler process. The native kernel LLVM
path owns that indirect call. This is an execution-safety boundary of the hosted
tool, not a missing native ABI.
