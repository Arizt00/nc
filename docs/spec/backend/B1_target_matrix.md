# nC - B1 Target Matrix

Status: Active Base (B1.2 + B1.8 + B1.9 + B1.12)
Version: v2.0.0-B1

## 1. Tier 1 (official first-class line)

| Profile | Triple | ISA | ABI | Object Format | Runtime Model | Tier | Backend Kind |
|---|---|---|---|---|---|---|---|
| `nC.sysv64` | `x86_64-unknown-linux-gnu` | x86_64 | SysV64 | ELF64 | Hosted | Tier1 | HostedProduction |
| `nC.win64` | `x86_64-pc-windows-msvc` | x86_64 | Win64 | COFF | Hosted | Tier1 | HostedProduction |
| `nC.aapcs64` | `aarch64-apple-darwin` | aarch64 | AAPCS64 | Mach-O | Hosted | Tier1 | HostedProduction |
| `nC.portable` | `wasm32-wasi` | wasm32 | WASI | wasm-module | Portable | Tier1 | Portable |

## 2. Tier 2 (declared next-line targets)

| Triple | ISA | Runtime Model | Tier |
|---|---|---|---|
| `aarch64-unknown-linux-gnu` | aarch64 | Hosted | Tier2 |
| `riscv64-unknown-linux-gnu` | riscv64 | Hosted | Tier2 |
| `x86_64-unknown-none-elf` | x86_64 | Freestanding | Tier2 |
| `riscv64-unknown-none-elf` | riscv64 | Freestanding | Tier2 |

## 3. Tier 3 (experimental lane)

Tier3 is reserved for:

- freestanding experiments;
- specialized firmware targets;
- sovereign backend expansions for deeply modal execution.

Baseical Tier3 sovereign profile:

| Profile | Triple | ISA | ABI | Object Format | Runtime Model | Tier | Backend Kind |
|---|---|---|---|---|---|---|---|
| `nC.sovereign` | `x86_64-unknown-sovereign-nc` | x86_64 | NcSovereign | freestanding-image | Sovereign | Tier3 | SovereignNc |

## 4. Rule

Any target without explicit tier assignment is not considered officially supported.
