# Mach-O x86_64 and arm64 final linking v1

## Imported object contract

The native object importer reads the segment name from each `section_64`
record. This is required for relocatable Clang objects whose enclosing
`LC_SEGMENT_64` name is empty. It classifies executable text, literal pools,
constant strings, data, zero-fill, DWARF and unwind sections independently.

Local section-ordinal relocations receive deterministic object-local symbols,
so equal ordinals in different objects cannot collide.

## Relocations

x86_64 supports signed RIP-relative, branch and unsigned 64-bit forms used by
Clang Mach-O objects. arm64 supports the material Clang family used by normal
code and the hosted runtime:

- `ARM64_RELOC_BRANCH26`;
- `ARM64_RELOC_PAGE21`;
- `ARM64_RELOC_PAGEOFF12`, decoded as ADD or scaled load/store;
- `ARM64_RELOC_UNSIGNED`;
- GOT page and page-offset forms.

Range, alignment, section and unsupported-instruction failures are diagnosed;
they are never silently truncated.

## Loader image

The finalizer emits `MH_EXECUTE`, `LC_MAIN`, `LC_LOAD_DYLINKER`,
`LC_LOAD_DYLIB`, `LC_DYLD_INFO_ONLY`, symbol/string tables, indirect symbols,
bind/lazy-bind metadata and an ad-hoc code signature. x86_64 imports use 6-byte
RIP-relative stubs. arm64 imports use 12-byte `ADRP/LDR/BR x16` stubs and
8-byte lazy pointers.

Cross builds verify final bytes and reports on any host. Actual loader
execution remains host evidence and therefore runs only on the corresponding
Intel or Apple Silicon macOS CI worker.
