# Machine Link COFF

COFF/PE profile integration is closed for `x86_64-pc-windows-msvc` machine artifacts. The profile covers PE32+ executable bytes, static library archives, relocations, import table/IAT metadata, `.reloc` base relocation records, ASLR flags, and `.pdata/.xdata` preservation reports.

`ncc machine build-exe <file.nm> --target x86_64-pc-windows-msvc --out app.exe` writes a native Windows PE32+ image directly. The writer emits `MZ`/`PE\0\0`, a Win64 optional header, `.text`, optional `.rdata`/`.data`, `.reloc`, console subsystem metadata, and `IMAGE_IMPORT_DESCRIPTOR`/IAT records for dynamic imports such as `KERNEL32.dll!ExitProcess`. Final `call-near32` relocations target local import thunks, and final `abs64` slots receive `IMAGE_REL_BASED_DIR64` base-relocation entries. The PE optional header sets `DYNAMIC_BASE`, `HIGH_ENTROPY_VA`, `NX_COMPAT`, and `TERMINAL_SERVER_AWARE`, so the resulting bytes are loader-facing PE bytes, not an `NCCLINK12` container.

The same PE finalizer is now wired into the standard high-level surface:
`ncc build <file.n> --cc nC.win64` emits the high-level COFF object, emits the
retained Machine bridge as an entry COFF object, imports both through the
Phase 12 native COFF reader, and writes the final PE32+ executable directly when
no explicit `--linker` is supplied.

Hand-written Machine sources can use the same standard surface:
`ncc build <file.nm> --out-dir .ncc --json` validates the `.nm`, emits `.obj`,
links `.exe`, and retains manifest/map/report artifacts.

Phase 12 accepts third-party COFF `.obj` relocatables without NCC `link_unit` metadata. The native importer reads section headers, the COFF symbol table, string table names, `IMAGE_REL_AMD64_REL32`, and `IMAGE_REL_AMD64_ADDR64` records, then reports `link_unit_origin: native_object`.

COFF symbol-table indices remain distinct while equal-named input sections are
coalesced. In particular, two `.rdata` contributions may each expose a local
section symbol named `.rdata`; the importer assigns each local entry an internal
identity before generic symbol resolution. Both the relocation patch offset and
the target address therefore include the selected input contribution base plus
the symbol value for `REL32` and `ADDR64`.

`ncc machine third-party-objects --json` verifies producer-side COFF rows for clang, MSVC `cl`, and MASM `ml64` when those tools are available. CI runs the matrix with `--strict-available`, so available COFF producers must generate importable `.obj` files with `IMAGE_REL_AMD64_REL32` and `IMAGE_REL_AMD64_ADDR64`.

Import-library compatibility remains outside Phase 12 unless declared by a later profile; the direct PE writer materializes imports without requiring a `.lib`.

The final Windows loader smoke is:
`ncc machine link-smoke examples/machine/native/link_final_windows_exit42.nm --target x86_64-pc-windows-msvc --system-loader --expect-exit 42 --json`.
It writes the Phase 12 PE32+ image, including imports, `.reloc` and ASLR flags,
then executes that exact `.exe` through the Windows loader. Non-Windows hosts
return a typed blocked state instead of counting byte validation as execution.
