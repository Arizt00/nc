# Machine Link ELF64

ELF64 linking is closed for `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, and `riscv64-unknown-elf` profiles.

The link report records ELF input hashes, section layout, symbol resolution, relocation application, debug/unwind preservation where present, map file hash, and artifact hash.

For executable phase12 artifacts, `machine link` and `machine build-exe` now emit an OS-loader executable (`ET_EXEC`) with section headers and permissioned `PT_LOAD` segments. The in-process final linker consumes phase12 `link_unit` metadata when present, or imports third-party ELF64 relocatable objects directly from `.symtab`, `.strtab`, section headers and `.rel/.rela*` records. It merges `.text`, `.rodata`, `.data`, `.bss`, unwind and debug sections, resolves cross-object symbols, respects `_start`/`main` entry symbols, and applies final relocations after layout.

Final ELF64 permissions are split as RX `.text/.plt`, read-only `.rodata/.eh_frame/.dyn*`, RW `.got/.dynamic/.data`, and `SHT_NOBITS` `.bss` represented by RW segment memory size rather than file bytes. `.debug_*` sections are preserved as non-allocated section headers outside loader segments.

The ELF64 final relocation matrix covers x86_64 `ABS64`, `ABS32`, `ABS32S`, `PC32`, `PLT32`, and GOTPCREL/GOTPCRELX; AArch64 `ABS64`, `ABS32`, `CALL26`, `JUMP26`, `ADRP`, low-12 ADD/LDST, and GOT page/low-12 pairs; and RISC-V `ABS64`, `ABS32`, branch, JAL, CALL/CALL_PLT pairs, HI20/LO12, PCREL HI/LO, GOT_HI20 with paired PCREL_LO12, and RELAX marker skipping. Static GOT users allocate final GOT slots and write resolved target addresses into those slots before `.text`. `.data` relocation records are also applied in the final executable image.

x86_64 dynamic imports additionally materialize `.hash`, `.dynstr`, `.dynsym`, `.rela.plt`, `.got.plt`, `.plt`, `.dynamic`, `PT_INTERP`, and `PT_DYNAMIC`. The contract is documented in [Machine Dynamic Unix Imports](machine_link_dynamic_unix.md). `NCCLINK12` remains the deterministic internal lane for non-final artifact shapes and unsupported executable writer targets.

The final ELF smoke is `ncc machine link-smoke examples/machine/native/link_final_linux_exit42.nm --target x86_64-unknown-linux-gnu --system-loader --expect-exit 42 --json`. It writes the phase12 ELF executable and executes it through the Linux kernel loader; success requires observed exit code `42`.

Third-party ELF evidence includes `machine_phase12_links_system_toolchain_elf_objects_without_ncc_link_unit`, which builds two `.o` files with the host assembler/compiler, links them without `.machine-link-meta.json`, and verifies that the final call relocation was applied by NCC.

The hard producer matrix is `ncc machine third-party-objects --json`. ELF rows cover clang, gcc, GNU `as` and nasm objects, including call/branch relocations, absolute data pointers, PC-relative data and GOTPCREL where the producer supports it.
