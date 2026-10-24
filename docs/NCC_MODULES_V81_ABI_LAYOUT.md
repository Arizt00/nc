# V81 native alignment

English edition of the retained V81 ABI and layout contract.

## Contract

`align(N)` retains its syntax and layout calculation. N must be a positive power of two. The native x86-64 backend no longer imposes an artificial 16-byte aggregate alignment limit.

This profile supports alignment up to 1 MiB. Temporary and stack aggregate values retain a 1 MiB material size limit per value; static storage has its own size limits. An unrepresentable alignment is rejected, never truncated or rounded down.

| Route | Storage alignment |
| --- | --- |
| AMD64 COFF | Powers of two up to the format limit of 8192 bytes |
| ELF64 ET_REL | Powers of two up to 1 MiB in this backend |
| Direct PE64 | Powers of two up to 1 MiB in this backend |
| Direct ELF64 | Powers of two up to 1 MiB in this backend |
| x86-64 locals, temporary arguments and returns | Up to the 1 MiB material limit |

A 64 KiB-aligned global can be materialized as direct PE64 but cannot be represented as a 64 KiB-aligned COFF section. That route must fail without writing an invalid object. Stack limits remain the program's responsibility; probing does not increase the reserve. The 1 MiB local test explicitly reserves 16 MiB when linking on Windows, rather than using the 1 MiB direct-PE reserve. The 1 MiB by-value argument/return test uses the same Windows reserve and a 32 MiB Linux stack limit.

When linking 8192-aligned COFF with MSVC, specify `/ALIGN:8192`. The linker rejects section alignment exceeding the requested image alignment. LLD accepts the object with default options. The direct PE route aligns data and BSS virtual addresses without an external linker.

## Stack and ABI

Each overaligned local has a reserved interval with room for address adjustment. The emitter aligns the address within that interval without changing rbp or overlapping neighboring slots.

Win64 continues passing required aggregate copies by address, preserving requested alignment. SysV aligns the argument area to the largest alignment of memory arguments, retains the previous rsp and restores it after the call. Arguments are evaluated before adjusting the area; ABI registers are loaded after copies are complete.

Large reservations and dynamic call adjustment touch each stack page before proceeding. rbp remains stable for DWARF and Win64 metadata. No allocator, heap or scheduler is introduced.

## Objects and images

MCT retains maximum storage alignment and each symbol's alignment. The reader validates powers of two, ranges and offsets. ELF carries sh_addralign; COFF encodes IMAGE_SCN_ALIGN_*. The internal linker computes data and BSS virtual addresses using the image base, not the host buffer address. Relocations retain the real symbol and addend.

V81 cache identity differs from V80. IR, MCT and objects emitted under the earlier contract must not be reused.

## External contract references

- [Microsoft PE/COFF](https://learn.microsoft.com/en-gb/windows/win32/debug/pe-format): section characteristics and image alignment.
- [x86-64 psABI](https://gitlab.com/x86-psABIs/x86-64-ABI/blob/master/x86-64-ABI/low-level-sys-info.tex): memory argument-area alignment.

No new ISA was needed. LEA, AND, TEST, comparisons, branches and stack operations already belong to x86-64. This work extends their selection, encoding and storage contracts.
