# Machine Roundtrip ELF64

ELF64 roundtrip covers x86_64 legacy, VEX, AVX2, EVEX and AVX-512 examples emitted by `[@machine]`.

The object-aware comparison reads `.text`, `.symtab`, `.strtab`, relocation sections, DWARF debug sections and `.eh_frame` when present.

ELF64 is closed for the official Phase 7 positive examples.
