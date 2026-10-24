# Machine Linker Overview

Phase 12 adds official nC machine linker integration. The primary lane is the internal controlled linker, which consumes objects emitted by `[@machine]`, resolves declared link inputs, lays out sections, applies final relocations, emits a linked artifact, writes a map file, and stores a reproducible link report.

For ELF64, PE64 and Mach-O executable outputs, `machine link` and `machine build-exe` now use the in-process final linker. The final linker consumes phase12 object metadata (`link_unit`) when present, and otherwise imports real native ELF64/COFF/Mach-O relocatable objects directly from their section, symbol and relocation tables. It merges `.text`/`.rodata`/`.data`/`.bss`, preserves unwind/debug sections, resolves cross-object symbols, applies final text/rodata/data relocations for the declared architecture, and writes OS-loader executable bytes directly. ELF64 final relocation coverage includes x86_64 GOTPCREL, AArch64 branch/page/low-12/GOT pairs, and RISC-V branch/JAL/CALL/HI20/LO12/PCREL/GOT pairs. PE64 final output includes a Win64 PE32+ optional header, import descriptor/IAT records, `.reloc` base relocation blocks, `IMAGE_REL_BASED_DIR64` entries for `abs64` slots, and ASLR flags. The deterministic `NCCLINK12` container remains the internal lane for non-final artifact shapes and targets that do not yet have an OS-loader writer.

Third-party object inputs do not need a `.machine-link-meta.json` sidecar. Link reports expose `link_unit_origin` as `ncc_link_unit`, `ncc_source_recovered`, `ncc_audit`, or `native_object` so evidence distinguishes NCC metadata from native object import.

Producer-side third-party coverage is tracked by `ncc machine third-party-objects --json`. The matrix generates host-applicable MSVC `cl`, clang, gcc, GNU `as`, nasm and MASM `ml64` objects, imports them without NCC sidecars, checks `native_object` origin, resolves cross-object symbols, and verifies aggressive relocation families. Missing producers are reported as `blocked_by_environment`; `--strict-available` fails if an available producer cannot produce/import/link its row.

System-loader smoke is separate from byte/report validation. `machine link-smoke --system-loader` writes the final executable and asks the host OS loader to run it; Linux ELF is locally executable on Linux x86_64, PE64 executes on a retained Windows x86_64 runner, and Mach-O executes on a retained macOS x86_64 runner.

The external adapter lane is documented but does not serve as closure authority in this phase. Phase 13 remains the place for external tool compatibility validation.

`asm_complete_total` remains true because Phase 12 extends the closed machine pipeline without reopening native ASM closure.
