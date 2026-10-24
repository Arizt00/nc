# Backend Atomic Host Smoke

`ncc backend-model --industrial --host-smoke --json` validates the Win64 and
AArch64 atomic RMW byte matrices with external target tools. It does not execute
the target binaries; it validates the object/link/disassembly path for every
matrix row.

For each row the smoke:

- writes a target assembly fixture under `target/tmp/backend-atomic-host-smoke`;
- runs the target assembler;
- checks the expected byte sequence inside the produced object;
- runs target `objdump -d`;
- verifies the expected instruction family appears in disassembly;
- links a target artifact with the target linker.

Toolchains used when available:

- Win64: `x86_64-w64-mingw32-as`, `x86_64-w64-mingw32-objdump`,
  `x86_64-w64-mingw32-ld`.
- AArch64: `aarch64-linux-gnu-as`, `aarch64-linux-gnu-objdump`,
  `aarch64-linux-gnu-ld`.

Missing tools are reported as `missing_environment`, never as success. The
current WSL closure host has both toolchains installed and passed all 60 rows:
30 Win64 rows and 30 AArch64 rows.

This smoke closes external assembler/disassembler/linker validation for the
current matrix rows. It does not close formal memory-order proof, byte/word RMW
coverage, native Windows execution, native AArch64 execution, or prolonged
multi-host CI.
