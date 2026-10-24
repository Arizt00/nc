# Machine static and automatic memory authority

`ncc-machine::static_memory` is the shared authority for storage and stack
materialization. Parser, IR, object writers and kernel lowering consume this
contract instead of duplicating target-specific names.

## Native section families

The authority maps data, BSS, read-only data, TLS, per-CPU templates,
constructor arrays, destructor arrays and custom sections to ELF, COFF, Mach-O
and freestanding image conventions. Each plan includes alignment, permissions,
TLS/zerofill state, linkage, COMDAT group and linker-GC eligibility.

## Dynamic stack ISA

The machine layer emits real bytes for fixed stack adjustment and the raw
mark/restore/page-touch primitives on:

- x86-64 and i386;
- AArch64;
- ARMv7 and Thumb;
- RV32 and RV64.

The target stack plan selects ABI alignment, page size, probe threshold, probe
strategy, guard-page owner and unwind encoding. High-level VLA lowering uses
LLVM/MCT when a dynamic frame must coexist with compiler-managed locals; this
keeps fixed slots frame-relative and still produces target machine code and a
native object rather than text or an interpreter fallback.

## Linker contract

ELF objects carry NOBITS/TLS/group section flags. COFF objects carry
uninitialized-data, alignment and COMDAT auxiliary records. Mach-O objects
carry zerofill, thread-local, coalesced and module init/term section types.
Lifecycle pointer slots contain native relocations to their function symbols.
