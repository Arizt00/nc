# nC v1.0.0 — Spec maestra

Fecha: 2026-04-30
Estado: oficial para release mayor v1.0.0

## 1. Propósito

Esta spec maestra fija el contrato global de nC v1.0.0 en la línea activa del repositorio.
Prioriza coherencia tipada end-to-end entre parser, AST, sema, IR y runtime/lowering/backend.

## 2. Jerarquía canónica

1. `docs/spec/release/v1_0_roadmap_obligatorio.md`
2. `docs/spec/release/v1_0_acceptance_matrix.md`
3. Especificaciones por dominio en `docs/spec/*`
4. `docs/handoff/ROADMAP_CODEX_MASTER.md`
5. `docs/handoff/CODEX_HANDOFF_2026-04.md`

Si dos documentos discrepan, prevalece el de mayor prioridad en esta lista.

## 3. Superficie oficial incluida en v1.0.0

- `[@]`
- `[@machine]`
- `primitive[@]`
- linking soberano
- memoria/punteros/regiones/alias
- runtime profundo base
- `[@inspect]`
- `[@security]`
- `[@network]`
- `[@system]`
- `[@tooling]`
- `[@compiler]`
- `[@kernel]`
- `[@ai]`
- backend material defendible
- matriz oficial de aceptación

## 4. Invariantes de implementación

- No degradar estructuras tipadas a rutas stringly en zonas ya tipadas.
- Diagnósticos canónicos estables y verificables por fixtures `FAIL`.
- Todo cierre de fase requiere evidencia reproducible en tests.
- Las ampliaciones deben mantener continuidad parser -> AST -> sema -> IR -> runtime/backend.

## 5. Documentación de versión mayor (F22)

Este cierre documental se compone de:

- `docs/spec/release/v1_0_spec_maestra.md`
- `docs/spec/release/v1_0_roadmap_master_sync.md`
- `docs/spec/release/v1_0_binary_closure_acta.md`
- `docs/spec/release/v1_0_install_usage_guide.md`
- `docs/spec/release/v1_0_modes_guide.md`
- `docs/spec/release/v1_0_build_run_guide.md`
- `docs/spec/release/v1_0_tooling_guide.md`
- `docs/spec/release/v1_0_handoff_third_parties.md`
- `docs/spec/release/v1_0_features_table.md`
- `docs/spec/release/v1_0_compatibility_table.md`
- `docs/spec/release/v1_0_stability_scope_table.md`
- `docs/spec/release/f24_release_engineering_v1_0.md`
- `docs/spec/release/v1_0_changelog.md`
- `docs/spec/release/v1_0_release_notes.md`

## 6. Evidencia mínima de release

- `cargo test --workspace -- --nocapture`
- Matriz de aceptación en estado coherente con implementación.
- Gates F21/F22/F23/F24 en tests de `ncc-cli` sin regresiones.

## 7. Política de compatibilidad relacionada

La congelación formal de compatibilidad 1.x se define en:

- `docs/spec/release/v1_0_compatibility_policy_1x.md`
- `docs/spec/release/v1_0_compatibility_contract.json`

## 8. Anexo de congelación de lenguaje v1.0.1 (Fase 1)

Para la etapa de cierre estricto de base de lenguaje en `1.x` se añade:

- `docs/spec/release/v1_0_1_language_spec_immutable.md`
- `docs/spec/release/v1_0_1_compatibility_spec.md`
- `docs/spec/release/v1_0_1_language_freeze_contract.json`
- `docs/spec/release/rfc/RFC_TEMPLATE.md`

Regla activa:

- no changes breaking without RFC aprobado.

## 9. Anexo de modelo de toolchains v1.0.1 (Fase 2)

Para la etapa de cierre estricto de distribución/instalación del ecosistema en `1.x` se añade:

- `docs/spec/release/v1_0_1_toolchain_model_spec.md`
- `docs/spec/release/v1_0_1_toolchain_compatibility_spec.md`
- `docs/spec/release/v1_0_1_toolchain_model_contract.json`
- `docs/status/STATUS_PHASE2_toolchain_model_v1_0_1.md`

Regla activa:

- no changes breaking del modelo de toolchains sin RFC aprobado.

## 10. Anexo de runtime + stdlib v1.0.1 (Fase 3)

Para la etapa de cierre estricto de runtime/stdlib de producción en `1.x` se añade:

- `docs/spec/release/v1_0_1_runtime_stdlib_spec.md`
- `docs/spec/release/v1_0_1_runtime_stdlib_compatibility_spec.md`
- `docs/spec/release/v1_0_1_runtime_stdlib_contract.json`
- `docs/status/STATUS_PHASE3_runtime_stdlib_v1_0_1.md`

Regla activa:

- no changes breaking de runtime/stdlib base sin RFC aprobado.

## 11. Anexo de backend soberano + ABI v1.0.1 (Fase 4)

Para la etapa de cierre estricto de backend de producción en `1.x` se añade:

- `docs/spec/release/v1_0_1_backend_sovereign_spec.md`
- `docs/spec/release/v1_0_1_backend_sovereign_compatibility_spec.md`
- `docs/spec/release/v1_0_1_backend_sovereign_contract.json`
- `docs/status/STATUS_PHASE4_backend_sovereign_v1_0_1.md`

Regla activa:

- no changes breaking de backend/ABI/data-layout/object-format sin RFC aprobado.

## 12. Anexo de toolchain completa v1.0.1 (Fase 5)

Para la etapa de cierre estricto de tooling de producción en `1.x` se añade:

- `docs/spec/release/v1_0_1_toolchain_complete_spec.md`
- `docs/spec/release/v1_0_1_toolchain_complete_compatibility_spec.md`
- `docs/spec/release/v1_0_1_toolchain_complete_contract.json`
- `docs/status/STATUS_PHASE5_toolchain_complete_v1_0_1.md`

Regla activa:

- no changes breaking de la UX/ayuda/errores del tooling oficial sin RFC aprobado.

## 13. Anexo de calidad extrema v1.0.1 (Fase 6)

Para la etapa de cierre estricto de hardening/QA en `1.x` se añade:

- `docs/spec/release/v1_0_1_quality_extreme_spec.md`
- `docs/spec/release/v1_0_1_quality_extreme_compatibility_spec.md`
- `docs/spec/release/v1_0_1_quality_extreme_contract.json`
- `docs/status/STATUS_PHASE6_quality_extreme_v1_0_1.md`

Regla activa:

- no changes breaking en fuzz/bench/security regression gates sin RFC aprobado.

## 14. Anexo de ecosistema de paquetes y builds v1.0.1 (Fase 7)

Para la etapa de cierre estricto de dependencias/lockfile/registry en `1.x` se añade:

- `docs/spec/release/v1_0_1_package_ecosystem_spec.md`
- `docs/spec/release/v1_0_1_package_ecosystem_compatibility_spec.md`
- `docs/spec/release/v1_0_1_package_ecosystem_contract.json`
- `docs/status/STATUS_PHASE7_package_ecosystem_v1_0_1.md`

Regla activa:

- no changes breaking de resolver/semver/lock/registry sin RFC aprobado.

## 15. Anexo de DX y observabilidad v1.0.1 (Fase 8)

Para la etapa de cierre estricto de DX/troubleshooting en `1.x` se añade:

- `docs/spec/release/v1_0_1_dx_observability_spec.md`
- `docs/spec/release/v1_0_1_dx_observability_compatibility_spec.md`
- `docs/spec/release/v1_0_1_dx_observability_contract.json`
- `docs/status/STATUS_PHASE8_dx_observability_v1_0_1.md`

Regla activa:

- no changes breaking de códigos estables/sugerencias/schema de perfiles sin RFC aprobado.

## 16. Anexo de gobernanza y ciclo de releases v1.0.1 (Fase 9)

Para la etapa de cierre estricto de modelo institucional de releases en `1.x` se añade:

- `docs/spec/release/v1_0_1_governance_release_spec.md`
- `docs/spec/release/v1_0_1_governance_release_compatibility_spec.md`
- `docs/spec/release/v1_0_1_governance_release_contract.json`
- `docs/status/STATUS_PHASE9_governance_release_v1_0_1.md`

Regla activa:

- no changes breaking del release train, deprecaciones/LTS o soporte empresarial sin RFC aprobado.
