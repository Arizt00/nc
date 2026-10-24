# nC Toolchain Model Spec v1.0.1

Fecha de congelación: 2026-05-02  
Estado: `frozen`  
Ámbito: distribución de toolchains, componentes e instalación en serie `1.x`.

## 1. Objetivo

Congelar un modelo de toolchains equivalente al estándar operativo de lenguajes maduros.

Este spec fija de forma inmutable:

- Canales oficiales.
- Toolchains por target.
- Componentes obligatorios.
- Perfiles de instalación.
- Reglas de compatibilidad en `1.x`.

## 2. Canales oficiales

Canales congelados:

- `stable`
- `beta`
- `nightly`

Política:

- `stable`: superficie de máxima estabilidad (Tier 1).
- `beta`: validación previa de cambios no breaking (Tier 1 + Tier 2).
- `nightly`: validación de frontera y targets experimentales (Tier 1 + Tier 2 + Tier 3).

## 3. Toolchains por target

Targets canónicos del modelo:

- `x86_64-unknown-linux-gnu` (tier1)
- `x86_64-pc-windows-msvc` (tier1)
- `aarch64-apple-darwin` (tier1)
- `wasm32-wasi` (tier1)
- `aarch64-unknown-linux-gnu` (tier2)
- `x86_64-unknown-none-elf` (tier3)
- `aarch64-unknown-none-elf` (tier3)
- `x86_64-unknown-sovereign-nc` (tier3)

Formato de identificador congelado:

- `<channel>-<target>`

Ejemplos canónicos:

- `stable-x86_64-unknown-linux-gnu`
- `stable-wasm32-wasi`
- `beta-aarch64-unknown-linux-gnu`
- `nightly-x86_64-unknown-sovereign-nc`

## 4. Componentes obligatorios

Componentes congelados en la familia toolchain:

- `compiler`
- `runtime`
- `stdlib`
- `fmt`
- `lint`
- `lsp`
- `docs`
- `sources`

## 5. Perfiles de instalación

Perfiles oficiales:

- `minimal`
- `default`
- `complete`

Composición congelada:

- `minimal`: `compiler`, `runtime`, `stdlib`
- `default`: `minimal` + `fmt`, `lint`, `lsp`, `docs`
- `complete`: `default` + `sources`

## 6. Contrato machine-readable

Contrato vinculante:

- `docs/spec/release/v1_0_1_toolchain_model_contract.json`

Campos críticos:

- `version = 1.0.1`
- `state = frozen`
- `series = 1.x`
- `channels = [stable, beta, nightly]`
- `components = [compiler, runtime, stdlib, fmt, lint, lsp, docs, sources]`
- `install_profiles = {minimal, default, complete}`

## 7. Integración CLI

Comando canónico:

- `ncc toolchain-model --json`
- `ncc toolchain-model --list`
- alias: `ncc toolchain`

Regla:

- la CLI no inventa toolchains fuera de este contrato.

## 8. Gate y cierre de fase

Gate obligatorio:

- `release_phase2_toolchain_model_gate_tests`

Fase 2 queda cerrada solo si:

1. este spec permanece en `frozen`;
2. el contrato JSON permanece en `frozen`;
3. el gate `release_phase2_toolchain_model_gate_tests` está en verde;
4. parser/sema/IR/backend mantienen continuidad en perfiles canónicos del modelo.
