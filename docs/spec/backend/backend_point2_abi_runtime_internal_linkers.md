# Backend Point 2 - ABI, runtime imports and internal OS linkers

Status: implemented for the normal high-level `.n` build path on the supported OS executable targets. Targets whose direct high-level backend object writer is not native yet use the standard MCT-machine body route: high-level `.n` lowers to optimized target LLVM IR, materializes a real target object, keeps a machine `.nm` entry bridge, and links both inputs with the internal OS executable linker.

## Closed

- `ncc build` / `ncc nbuild` now selects the internal machine OS linker by object format when no explicit `--linker` is passed.
- The normal `.n` path emits the C-like artifact set through the native backend:
  `.optimizer.json`, `.backend-object.nbo`, `.nm`, `.obj`, `.entry.obj`, `.exe`.
- Supported internal OS executable formats from normal `.n`:
  - PE64/COFF for `x86_64-pc-windows-msvc`.
  - ELF64 for `x86_64-unknown-linux-gnu`.
  - ELF64 for `aarch64-unknown-linux-gnu`.
  - ELF64 for `riscv64-unknown-linux-gnu` through the standard MCT-machine body route.
  - ELF64 for `riscv64-unknown-elf` through the standard MCT-machine body route when the image is freestanding/autocontained.
  - ELF32 for `i386-unknown-linux-gnu` through the standard MCT-machine body route.
  - ELF32 for `riscv32imac-unknown-elf`, canonicalized to `riscv32-unknown-elf` with `rv32imac` code generation, through the standard MCT-machine body route.
  - Mach-O for `x86_64-apple-darwin`.
- The build manifest records the ABI contract visible at backend-object level:
  ABI model, pointer width, stack alignment, aggregate alignment, aggregate return contract and direct machine call lowering.
- Runtime imports visible in the backend object are now reported under
  `machine_backend.runtime_import_materialization`.
- ELF64 x86_64 runtime imports are materially linked by the internal OS linker through real dynamic metadata:
  `.dynstr`, `.dynsym`, `.rela.plt`, `.plt`, `.got.plt` and `.dynamic`.
- ELF64 AArch64 and RISC-V64 Linux runtime imports are materially linked by the internal OS linker through target-specific interpreter paths, PLT bytes, GOT/PLT slots, `.rela.plt`, `DT_BIND_NOW`, `DT_FLAGS` and architecture-specific jump-slot relocations.
- ELF32 i386 runtime imports are materially linked by the internal OS linker through real dynamic metadata:
  `.interp`, `.dynstr`, `.dynsym`, `.hash`, `.rel.plt`, `.plt`, `.got.plt` and `.dynamic`.
- PE64 keeps the existing internal import-table/IAT route and internal runtime thunks for the supported runtime surface.
- `--target` on normal `.n` is now enforced: if the high-level backend emits a different target triple, the build fails instead of producing a mislabeled executable.
- i386/RISC-V standard builds reject explicit `--linker` and use Phase12 internally so the final executable route is not delegated to an external host linker.
- i386/RISC-V standard builds retain `.standard.mct.ll`, `.backend-object.nbo`, `.nm`, `.obj`, `.entry.obj`, `.exe`, launchers, map and manifest evidence.

## Runtime import materialization contract

The standard build manifest now distinguishes:

- `closed_no_runtime_imports_requested`: autocontained program object.
- `closed_by_internal_machine_linker`: runtime symbol is unresolved at object stage and resolved/materialized by Phase12 internal OS linker.
- `delegated_to_configured_linker`: user explicitly passed `--linker`, so ncc does not pretend the internal linker handled it.

The smoke fixtures verify external runtime output imports through the internal ELF dynamic import route. Depending on target and runtime lowering, the imported symbol is visible as `printf` or the lower-level `write` entry in the imported native object.

For the standard MCT-machine body route, runtime imports are visible to machine/Phase12 through the imported native object plus the generated `.nm` bridge. If a target asks for dynamic imports that its executable writer does not implement yet, the build fails with a precise linker diagnostic instead of pretending the import was materialized.

## Target honesty boundary

The normal `.n` path no longer labels x86_64 bytes as i386/RISC-V output. For i386/RISC-V it uses target-specific MCT LLVM IR, target object materialization and the machine internal OS linker. Missing object materialization tools are reported as a real failure; they are not simulated.

## Pending next

- Finish Mach-O arm64 runtime imports with real bind/lazy-bind metadata and target stubs.
- Keep RV32 `unknown-elf` dynamic imports rejected unless a concrete OS dynamic-loader ABI target is selected.
- Expand runtime thunks/import adapters beyond the proven `printf`/PE runtime families into the full standard runtime catalog.
- Replace the MCT body object dependency with direct high-level machine instruction selection on targets where the native direct backend is promoted to first-class.
