# Machine Point 2 - Internal OS executable linkers

Status: Phase12 is now used by the normal `.n` build path for supported OS executable formats.

## Closed in machine/linker

- Normal `.n` builds without `--linker` call Phase12 through `link_machine_object_paths_to_os_executable_silent`.
- Phase12 finalizes real loader artifacts for:
  - `elf64`
  - `elf32`, including normal `.n` standard MCT-machine body builds for i386 and RV32
  - `coff` / PE64
  - `macho`
- `machine verify-link` recognizes real ELF32 OS executables in addition to ELF64, PE64 and Mach-O64.
- Link-unit relocation accounting is architecture-aware for generated `.nm` bridges, so RISC-V calls no longer use x86 byte lengths or x86 call-site addends.
- RISC-V bridge `call` relocations are applied as AUIPC/JALR call pairs in final executables.
- ELF32 i386 dynamic imports are finalized as loader metadata, not just a report:
  `.interp`, `.dynstr`, `.dynsym`, `.hash`, `.rel.plt`, `.plt`, `.got.plt`, `.dynamic`, `PT_INTERP`, `PT_DYNAMIC`, `DT_REL`, `DT_RELSZ`, `DT_RELENT` and `R_386_JMP_SLOT`.
- ELF64 AArch64 and RISC-V64 Linux dynamic imports are finalized as loader metadata with target-specific `.interp`, `.dynstr`, `.dynsym`, `.rela.plt`, `.plt`, `.got.plt`, `.dynamic`, `PT_INTERP`, `PT_DYNAMIC`, `DT_RELA`, `DT_RELASZ`, `DT_RELAENT`, `DT_BIND_NOW`, `DT_FLAGS` and the architecture jump-slot relocations.
- RISC-V assembler marker/relax relocations are retained in evidence as ignored `none` relocs and no longer create unresolved empty symbols during final linking.
- Mach-O finalization now accepts autocontained AArch64 images at header/finalizer level and emits the correct arm64 CPU type.
- Mach-O dynamic imports on AArch64 remain guarded with an explicit diagnostic until target-specific stubs and bind metadata are implemented.
- Hosted machine entry bridges now retain `macho_unwind: compact`.
- Windows extended paths are normalized before being embedded in `.nm` debug/evidence strings, avoiding invalid `//?/C:/...` literals.

## Linker behavior

If no `--linker` is provided:

- `coff` uses `native-pe64-internal`.
- `elf64` uses `native-elf64-internal`.
- `elf32` uses `native-elf32-internal`.
- `macho` uses `native-macho-internal`.

If `--linker` is provided, ncc keeps the configured host-linker route.

For the standard MCT-machine targets (`i386-unknown-linux-gnu`, `riscv32imac-unknown-elf`, `riscv64-unknown-elf`) the normal `.n` build path requires the internal linker and rejects `--linker`. The body object is a real target object imported by Phase12, and the retained `.nm` bridge defines the process entry symbol that calls the materialized high-level body.

## Evidence

The linked executable report is retained in the standard build manifest under:

```text
link.phase12_report
```

The report includes:

- input object hashes;
- resolved and unresolved symbols;
- dynamic import plan;
- relocations applied/failed;
- final section layout;
- OS loader artifact magic;
- map file hash and path.

## Pending next

- Make Mach-O native-object symbol import/export parity preserve cross-input high-level symbols without using a lazy-bind-looking report entry.
- Keep RV32 dynamic imports gated until an OS target with a real dynamic-loader contract is admitted; `riscv32imac-unknown-elf` remains freestanding.
- Implement Mach-O arm64 dynamic import stubs plus bind/lazy-bind metadata.
- Add loader execution smoke on Linux/macOS CI for generated ELF/Mach-O, not just `verify-link`.
