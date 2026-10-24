# Machine Targets

| Target | Arch | ABI | Object | Status |
|---|---|---|---|---|
| `x86_64-unknown-linux-gnu` | x86_64 | sysv64 | elf64 | local link/execute closed |
| `x86_64-pc-windows-msvc` | x86_64 | win64 | coff/PE32+ | direct PE bytes, imports, `.reloc`, ASLR and Windows loader smoke gate configured |
| `x86_64-apple-darwin` | x86_64 | darwin | macho | object artifact closed; CI execute pending |
| `aarch64-unknown-linux-gnu` | aarch64 | aapcs64 | elf64 | smoke encoder + ELF64 artifact closed; native Linux aarch64 execution gate configured |
| `riscv64-unknown-elf` | riscv64 | riscv64 | elf64 | smoke encoder + ELF64 artifact closed; qemu-riscv64 execution gate configured |
| `wasm32-unknown-unknown` | wasm32 | wasm | wasm | module generation + Node execution closed locally |

Run `ncc machine targets --json` for the machine-readable matrix.

## Non-x86 Execution Gates

- Aarch64 release evidence requires `ncc machine link-smoke examples/machine/aarch64_add_two_aapcs64.nm --target aarch64-unknown-linux-gnu --json --strict` on a retained Linux aarch64 runner artifact.
- Riscv64 release evidence requires `ncc machine link-smoke examples/machine/riscv64_add_two_elf.nm --target riscv64-unknown-elf --json --strict` with `riscv64-linux-gnu-gcc` and `qemu-riscv64`.
- Win64 release evidence requires `ncc machine link-smoke examples/machine/native/link_final_windows_exit42.nm --target x86_64-pc-windows-msvc --system-loader --expect-exit 42 --json --strict` on a retained native Windows x86_64 runner artifact.
- Local x86_64 artifact generation is not enough to claim non-x86 execution closure.
