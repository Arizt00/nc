# Machine Dynamic Unix Imports

Status: closed for phase12 executable OS-loader artifacts and retained
internal metadata.

`ncc machine build-exe` and executable `ncc machine link` now emit real
loader-shaped ELF64/Mach-O executable artifacts for Unix targets. The final
linker consumes phase12 object `link_unit` metadata, merges code/data/BSS,
resolves cross-object symbols, applies final relocations, and writes map/report
evidence. `NCCLINK12` remains the deterministic internal lane for non-final
artifact shapes and targets without an OS-loader executable writer.

`ncc machine link-smoke --system-loader` executes the final phase12 executable
directly through the host OS loader. Linux x86_64 uses the kernel ELF loader and
is locally executable from WSL/Linux. Mach-O uses the native macOS x86_64 loader
and is accepted only on a retained macOS x86_64 runner; non-macOS hosts return a
typed blocked diagnostic instead of pretending execution happened.

## ELF64

For `elf64` executable/shared-library outputs with `extern:` imports, phase12
emits the executable bytes plus:

- `.dynstr` with `libc.so.6` and imported symbol names.
- `.dynsym` entries with `STB_GLOBAL` and `STT_FUNC`.
- `.rela.plt` entries using target jump-slot relocations.
- `.got.plt` slots.
- `.plt` lazy-binding stubs.
- `.hash` SysV lookup table.
- `.dynamic` entries including `DT_NEEDED`, `DT_HASH`, `DT_STRTAB`, `DT_SYMTAB`,
  `DT_PLTGOT`, `DT_JMPREL`, `DT_PLTRELSZ`, `DT_PLTREL`, `DT_RELA`,
  `DT_RELASZ`, `DT_RELAENT` and `DT_NULL`.
- `PT_LOAD`, `PT_INTERP` and `PT_DYNAMIC` program headers.

Each generated section carries byte length, SHA-256 hash and raw bytes in the
link report.

## Mach-O

For Mach-O executable/shared-library outputs with `extern:` imports, phase12
emits the executable bytes plus:

- `LC_LOAD_DYLIB` for `/usr/lib/libSystem.B.dylib`.
- `LC_LOAD_DYLINKER` for `/usr/lib/dyld`.
- `LC_MAIN` and `LC_BUILD_VERSION` for the modern dyld entry path.
- `LC_DYLD_INFO_ONLY` bind and lazy-bind metadata.
- `LC_SYMTAB` `nlist_64` records plus string table entries for defined and
  imported symbols.
- `LC_DYSYMTAB` indirect-symbol metadata.
- `LC_CODE_SIGNATURE` pointing at an ad-hoc SHA-256 CodeDirectory SuperBlob.
- `__TEXT,__stubs` bytes.
- `__DATA,__la_symbol_ptr` bytes.
- `__LINKEDIT` indirect symbol table, bind stream, lazy-bind stream,
  symbol/string tables and code-signature bytes.

The bind streams use dyld bind opcodes:
`SET_DYLIB_ORDINAL_IMM`, `SET_SYMBOL_TRAILING_FLAGS_IMM`, `SET_TYPE_IMM`,
`SET_SEGMENT_AND_OFFSET_ULEB` and `DO_BIND`.

## Evidence

Fixtures:

- `examples/machine/native/link_elf64_dynamic_imports.nm`
- `examples/machine/native/link_macho_dynamic_imports.nm`

Tests:

- `machine_phase12_elf64_dynamic_import_metadata_ok`
- `machine_phase12_macho_dynamic_bind_metadata_ok`
- `machine_phase12_elf64_build_exe_is_os_loader_artifact`
- `machine_phase12_macho_build_exe_is_os_loader_artifact`
- `machine_phase12_elf64_final_linker_applies_cross_object_reloc`
- `machine_phase12_macho_final_linker_applies_cross_object_reloc`
- `machine_linux_elf_system_loader_smoke_executes_final_executable_or_gates`
- `machine_macho_system_loader_smoke_executes_final_executable_or_gates`

Commands:

```bash
ncc machine build-exe examples/machine/native/link_elf64_dynamic_imports.nm --target x86_64-unknown-linux-gnu --out target/tmp/phase12_elf_dynamic_imports.exe --json
ncc machine build-exe examples/machine/native/link_macho_dynamic_imports.nm --target x86_64-apple-darwin --out target/tmp/phase12_macho_dynamic_imports.mach --json
ncc machine link-smoke examples/machine/native/link_final_linux_exit42.nm --target x86_64-unknown-linux-gnu --system-loader --expect-exit 42 --json
ncc machine link-smoke examples/machine/native/link_final_macho_exit42.nm --target x86_64-apple-darwin --system-loader --expect-exit 42 --json
```
