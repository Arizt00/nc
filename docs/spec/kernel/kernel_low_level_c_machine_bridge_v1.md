# Kernel low-level C and Machine bridge v1

## Authority

Kernel source remains normal compact `.n` and requires no kernel import. C-like
intrinsic names are typed high-level operations. Machine instructions remain in
the typed `.nm` layer; raw assembler text is not duplicated inside `.n`.

## Atomic and synchronization surface

The admitted physical targets expose `atomic32_load`, `atomic32_store`,
`atomic32_exchange`, `atomic32_compare_exchange`, `atomic32_fetch_add`,
`spin_try_lock`, `spin_lock` and `spin_unlock`. Loads use acquire ordering,
stores use release ordering, and read-modify-write operations use acquire and
release ordering.

x86/i386 lower through lock-prefixed atomics, AArch64 through exclusive/LSE
selection, and RV32/RV64 through the admitted A-extension forms. Machine byte
tests remain the ISA authority. wasm returns the explicit unsupported result
and does not pretend to own physical shared-memory atomics.

## MMIO and barriers

`mmio_read8/16/32/64` and `mmio_write8/16/32/64` use volatile scalar loads and
stores. The 128/256/512-bit family uses an explicit raw-buffer ABI:
`mmio_readN(mmio, destination)` and `mmio_writeN(mmio, source)`. LLVM retains
the accesses as volatile `i128`, `i256` and `i512` operations and legalizes
them for each target without a hidden heap buffer or SIMD requirement.

Wide block operations allow unaligned addresses, preserve every transferred
bit and are not atomic. `read_barrier` and `write_barrier` share the
architecture memory-barrier contract. The caller owns address validity,
ordering and device semantics, matching kernel C expectations.

## C memory compatibility

`memcpy`, `memset`, `memmove` and `memcmp` have C-compatible pointer and byte
count behavior. `memmove` chooses forward or backward copy for overlapping
ranges. No hosted libc import is needed in a freestanding kernel image.

## Evidence

`examples/11_machine/kernel_low_level_c_asm_bridge.n` invokes every family and
round-trips 128, 256 and 512 bits through explicit source and destination
buffers. The kernel build test compiles and final-links it for the eleven
declared targets, checks emitted LLVM atomics/MMIO/memory bodies, and requires
an empty unresolved-symbol set for each target.
