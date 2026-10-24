# nC Phase10 Toolchain Model Compatibility Spec v2.0.0

Estado: `frozen`  
Serie: `2.x`

## Compatibilidad activa

Este documento declara la compatibilidad formal entre:

- Modelo base Fase 2 (`v1_0_1_toolchain_model_contract.json`).
- Modelo operacional F10-P2 (`v2_0_0_phase10_toolchain_model_contract.json`).

Reglas:

1. Canales `stable|beta|nightly` DEBEN mantenerse.
2. Componentes `compiler|runtime|stdlib|fmt|lint|lsp|docs|sources` DEBEN mantenerse.
3. Perfiles `minimal|default|complete` DEBEN mantenerse.
4. Formato `<channel>-<target>` DEBE mantenerse.
5. No se permiten breaking changes sin RFC formal.

## Continuidad CLI

- `toolchain-model` (1.x) se mantiene para continuidad de serie.
- `phase10-toolchain-model` / `toolchain-v2` se añade para cierre 2.x.
- `ncup toolchain install` mantiene forma por `<id>` y añade selector equivalente `--channel + --target`.

## Gates requeridos

- `release_phase10_toolchain_model_gate_tests`
- `release_phase10_installer_gate_tests`
- `release_phase2_toolchain_model_gate_tests`

## Frontera de alcance

- `owned_scope_artifacts` de F10-P2 PUEDEN evolucionar dentro de la serie 2.x bajo regla RFC.
- `locked_external_artifacts` (reportes F10-P1) NO DEBEN cambiar durante `qa_phase10_p2_v2_0_0.sh`.
