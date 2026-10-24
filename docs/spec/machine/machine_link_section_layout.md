# Machine Link Section Layout

Section layout is deterministic. Link reports include section source, address, size, alignment, permissions, and status.

The Phase 12 layout profile covers `.text`, `.rodata`, `.data`, `.bss`, debug/unwind sections, custom sections, and WASM module code regions where applicable.

For ELF64 final executables the layout is material, not only reported:

| Section family | ELF section contract | Segment permissions |
|---|---|---|
| `.text`, `.plt` | `SHT_PROGBITS`, `SHF_ALLOC | SHF_EXECINSTR` | `PT_LOAD` read/execute |
| `.rodata`, `.dynstr`, `.dynsym`, `.rela.plt`, `.eh_frame` | read-only allocated sections | `PT_LOAD` read-only |
| `.got`, `.got.plt`, `.dynamic`, `.data` | writable allocated sections | `PT_LOAD` read/write |
| `.bss` | `SHT_NOBITS`, writable allocated size | included in RW `memsz`, not file bytes |
| `.debug_*` | preserved non-allocated section headers and bytes | outside loader segments |

Section headers are emitted for final ELF executables so tooling can inspect `.text`, `.rodata`, `.data`, `.bss`, unwind and debug payloads after linking.
