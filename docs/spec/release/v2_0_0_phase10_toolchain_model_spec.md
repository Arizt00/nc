# nC Phase10 Toolchain Model Spec v2.0.0

Fecha de congelación: 2026-05-03  
Estado: `frozen`  
Serie: `2.x`  
Fase: `phase10_toolchain_model`

## 1. Objetivo

Cerrar el modelo de toolchains de nC para F10-P2 con contrato ejecutable, compatible con `ncup` y mantenido sin breaking changes fuera de RFC.

Este base DEBE fijar de extremo a extremo:

- canales oficiales;
- toolchains por target;
- componentes canónicos;
- perfiles de instalación;
- formato de identificador de toolchain;
- integración operativa con `ncup-init` y `ncup`.

## 2. Canales oficiales

Canales congelados:

- `stable`
- `beta`
- `nightly`

Regla canónica:

- Tier1 DEBE exponer `stable|beta|nightly`.
- Tier2 DEBE exponer `beta|nightly`.
- Tier3 DEBE exponer `nightly`.

## 3. Toolchains por target

Targets canónicos:

- `x86_64-unknown-linux-gnu` (tier1)
- `x86_64-pc-windows-msvc` (tier1)
- `aarch64-apple-darwin` (tier1)
- `wasm32-wasi` (tier1)
- `aarch64-unknown-linux-gnu` (tier2)
- `x86_64-unknown-none-elf` (tier3)
- `aarch64-unknown-none-elf` (tier3)
- `x86_64-unknown-sovereign-nc` (tier3)

Formato de ID congelado:

- `<channel>-<target>`

Ejemplos canónicos:

- `stable-x86_64-unknown-linux-gnu`
- `stable-wasm32-wasi`
- `beta-aarch64-unknown-linux-gnu`
- `nightly-x86_64-unknown-sovereign-nc`

## 4. Componentes oficiales

Componentes congelados:

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

Composición canónica:

- `minimal`: `compiler`, `runtime`, `stdlib`
- `default`: `minimal` + `fmt`, `lint`, `lsp`, `docs`
- `complete`: `default` + `sources`

## 6. Integración con `ncup`

`ncup` DEBE actuar como autoridad operativa de instalación sobre este modelo.

Flujo mínimo requerido:

1. `ncup-init --profile <minimal|default|complete>`
2. `ncup toolchain list --available`
3. `ncup toolchain install <channel-target-id> [--profile ...] [--set-default]`
4. `ncup toolchain install --channel <stable|beta|nightly> --target <triple> [--profile ...] [--set-default]`
5. `ncup toolchain default <id>`
6. `ncup doctor`

Regla:

- `ncup` NO DEBE instalar IDs fuera del modelo canónico.
- Cualquier ID desconocido DEBE fallar con diagnóstico accionable.

## 7. Superficies CLI canónicas

- `ncc toolchain-model --json|--list` (serie 1.x congelada)
- `ncc phase10-toolchain-model --json|--list` (serie 2.x congelada)
- alias: `ncc toolchain-v2 --json|--list`

## 8. Contratos y reportes

Contrato machine-readable:

- `docs/spec/release/v2_0_0_phase10_toolchain_model_contract.json`

Reporte de conformidad:

- `docs/status/F10_P2_toolchain_model_report.json`

Script de snapshot:

- `tools/release/snapshot_phase10_toolchain_model_v2_0_0.sh`

## 9. Gate de cierre

Gate obligatorio:

- `release_phase10_toolchain_model_gate_tests`
- `release_phase10_release_train_gate_tests`
- `tools/release/qa_phase10_p2_v2_0_0.sh` con aislamiento F10-P2→F10-P1

Cierre real de F10-P2 exige:

1. spec + compatibility + contract en estado `frozen`;
2. `phase10-toolchain-model` operativo;
3. reporte de conformidad válido;
4. gate en verde.

## 10. Frontera de alcance (cierre de puntos pendientes)

F10-P2 DEBE declarar frontera explícita:

- **Owned scope F10-P2**: spec/compat/contract/reporte/scripts de toolchain model + gates de release train 2.x.
- **Locked external scope F10-P1**: reportes de migración/distribución/install limpio que NO DEBEN mutar durante `qa_phase10_p2_v2_0_0.sh`.

Regla operativa:

- El script `qa_phase10_p2_v2_0_0.sh` calcula `sha256` antes y después de la suite y PROHÍBE cambios en artefactos bloqueados de F10-P1.
