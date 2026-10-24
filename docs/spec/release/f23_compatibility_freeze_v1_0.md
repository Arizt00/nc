# nC v1.0.0 — F23 Congelación de compatibilidad 1.x

Fecha de cierre: 2026-04-30
Estado: `SI`

## 1. Objetivo

Cerrar formalmente la política de compatibilidad de la serie `1.x`.

## 2. Entregables de cierre

- Política humana oficial:
  - `docs/spec/release/v1_0_compatibility_policy_1x.md`
- Contrato machine-readable:
  - `docs/spec/release/v1_0_compatibility_contract.json`
- Sincronización de estado:
  - `docs/spec/release/v1_0_acceptance_matrix.md` (F23 en `SI`/`SI`)
  - `docs/spec/release/v1_0_roadmap_obligatorio.md` (F23 cerrado)
  - `docs/spec/release/v1_0_roadmap_master_sync.md` (F23 `SI`)

## 3. Gate ejecutable

- `release_f23_compatibility_gate_tests::*` en `crates/ncc-cli/src/main.rs` valida:
  - existencia de artefactos F23;
  - estructura y contenido de política 1.x;
  - validez del contrato JSON;
  - consistencia matriz/roadmap.

## 4. Regla binaria

F23 solo permanece cerrado mientras:

1. el contrato `v1_0_compatibility_contract.json` se mantenga en estado `frozen`;
2. la fila F23 en matriz siga en `SI`/`SI`;
3. el roadmap mantenga F23 marcado como cerrado;
4. el gate F23 siga en verde.

## 5. Relación con F24

Con F24 cerrado, la release mayor `v1.0.0` queda completa en cierre binario F12..F24.

## 6. Continuidad con Fase 1 de base de lenguaje (v1.0.1)

La congelación F23 se extiende con el freeze específico de superficie de lenguaje:

- `docs/spec/release/v1_0_1_language_spec_immutable.md`
- `docs/spec/release/v1_0_1_compatibility_spec.md`
- `docs/spec/release/v1_0_1_language_freeze_contract.json`

Gate vinculado:

- `release_phase1_language_base_gate_tests`

## 7. Continuidad con Fase 2 de modelo de toolchains (v1.0.1)

La congelación F23 también cubre la superficie de distribución/instalación de toolchains:

- `docs/spec/release/v1_0_1_toolchain_model_spec.md`
- `docs/spec/release/v1_0_1_toolchain_compatibility_spec.md`
- `docs/spec/release/v1_0_1_toolchain_model_contract.json`

Gate vinculado:

- `release_phase2_toolchain_model_gate_tests`
