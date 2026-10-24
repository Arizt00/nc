# nC Backend Sovereign + ABI Spec v1.0.1

Estado: `frozen`  
Serie: `1.x`  
Fase: `phase4_backend_sovereign`

## Objetivo

Cerrar backend soberano y API reales de producción para:

- carriles backend (`hosted`, `portable`, `freestanding`, `sovereign`)
- ABI reales
- calling conventions canónicas
- data layout formal
- object formats reproducibles

## Matriz target tiered oficial

La política oficial se congela como matriz `Tier 1/2/3`.

Tier 1:

- `x86_64-unknown-linux-gnu` (`nC.sysv64`, ELF64)
- `x86_64-pc-windows-msvc` (`nC.win64`, COFF)
- `aarch64-apple-darwin` (`nC.aapcs64`, Mach-O)
- `wasm32-wasi` (`nC.portable`, wasm-module)

Tier 2:

- `aarch64-unknown-linux-gnu`

Tier 3:

- `x86_64-unknown-none-elf`
- `aarch64-unknown-none-elf`
- `x86_64-unknown-sovereign-nc` (`nC.sovereign`)

## Calling conventions y ABI canónicos

- `nC.sysv64` -> `SysV64`
- `nC.win64` -> `Win64`
- `nC.aapcs64` -> `AAPCS64`
- `nC.portable` -> `WASI`
- `nC.sovereign` -> `NcSovereign`

## Data layout canónico

Contracto tipado en `BackendDataLayoutIR`:

- `pointer_bytes`
- `word_bytes`
- `stack_align_bytes`
- `integer_align_bytes`
- `float_align_bytes`
- `aggregate_align_bytes`

## Object formats obligatorios

- `ELF64`
- `COFF`
- `Mach-O`
- `WASM module`

## Contratos de lowering por target

Referencias normativas:

- `docs/spec/backend/abi_lowering_contracts_v0_1.md`
- `docs/spec/backend/B1_legalization_matrix.md`
- `docs/spec/backend/B1_target_matrix.md`
- `docs/spec/backend/B1_linking_model.md`
- `docs/spec/backend/B1_backend_sovereignty_closure_spec.md`

## Criterios de producción

- semántica tipada end-to-end parser/sema/IR/backend;
- validación de lane/ABI/artefacto por contrato;
- emisiones reproducibles byte a byte por formato bajo mismo input/profile.

## Gate obligatorio

- `release_phase4_backend_sovereign_gate_tests`

## DoD

Fase 4 se considera cerrada solo si:

1. matriz target tiered + contratos de lowering quedan congelados;
2. ABI/calling convention/data layout quedan formalizados;
3. emisiones reproducibles ELF/COFF/Mach-O/WASM quedan validadas por tests de conformidad.
