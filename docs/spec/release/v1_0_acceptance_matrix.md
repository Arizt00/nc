# nC v1.0.0 — Matriz oficial de aceptación

Fecha de apertura: 2026-04-30

Estados permitidos por celda:

- `SI`: capa cerrada para la fase/modo.
- `NO`: capa abierta.
- `PARCIAL`: implementación activa pero no cerrada.
- `N/A`: no aplica al modo/fase.

## 1. Gate transversal previo (bloque inferior obligatorio)

| Ítem | Estado | Evidencia mínima requerida |
|---|---|---|
| Operadores tipados end-to-end | SI | parser/sema/runtime/lowering con matriz de operadores y precedencia congelada |
| Expansión ISA `[@machine]` | SI | AST/parser/sema/IR/backend en `mul/adc/sbb/sar/xchg/neg.bin` + tests positivos/negativos + emisión nativa x86_64 |
| Profundización radical `primitive[@]` | SI | parser/AST/sema/IR + materialización backend/object de contratos tipados (`capability/bitfield/operation/signal/port/signature/entry`) sin fallback stringly + tests CLI/IR |
| Multi-target ambicioso | SI | x86_64 + aarch64 en ABI/lowering/ELF64 tipado + `emit-coff`/`emit-macho` con ruta `objcopy` y fallback `clang` tipado validados por tests |
| Tooling sofisticado | SI | contratos ejecutables + bins dedicados `nfmt/nlint/nbuild/npkg` + matriz runtime/sema/parser F16 |
| Nuevas familias `use/import` | PARCIAL | catálogo ampliado + autoridad modal + tests |
| Optimizaciones avanzadas | NO | plan + pases IR + pruebas de no regresión |
| Ecosistema IDE profundo | NO | especificación mínima + integración verificable |

## 2. Matriz por modo/capa (v1.0.0)

| Modo/Fase | Parser | AST | Sema | IR | Runtime/Lowering/Backend | Fixtures OK | Fixtures FAIL | Docs | Cierre binario |
|---|---|---|---|---|---|---|---|---|---|
| `[@]` (F1 base + endurecimiento v1) | SI | SI | SI | SI | SI | SI | SI | SI | SI |
| `[@machine]` (F2 + expansión ISA v1) | SI | SI | SI | SI | SI | SI | SI | SI | SI |
| `primitive[@]` (F3 + profundización v1) | SI | SI | SI | SI | SI | SI | SI | SI | SI |
| Linking soberano (F4) | SI | SI | SI | SI | SI | SI | SI | SI | SI |
| Memoria/punteros/regiones (F5) | SI | SI | SI | SI | SI | SI | SI | SI | SI |
| Runtime profundo base (F6) | SI | SI | SI | SI | SI | SI | SI | SI | SI |
| `[@inspect]` (F7) | SI | SI | SI | SI | SI | SI | SI | SI | SI |
| `[@security]` (F13 operativo completo) | SI | SI | SI | SI | SI | SI | SI | SI | SI |
| `[@network]` (F14 ejecutable completo) | SI | SI | SI | SI | SI | SI | SI | SI | SI |
| `[@system]` (F15) | SI | SI | SI | SI | SI | SI | SI | SI | SI |
| `[@tooling]` (F16) | SI | SI | SI | SI | SI | SI | SI | SI | SI |
| `[@compiler]` (F17) | SI | SI | SI | SI | SI | SI | SI | SI | SI |
| `[@kernel]` (F18) | SI | SI | SI | SI | SI | SI | SI | SI | SI |
| `[@ai]` (F19 decisión oficial) | SI | SI | SI | SI | SI | SI | SI | SI | SI |
| Backend material defendible (F20) | SI | SI | SI | SI | SI | SI | SI | SI | SI |
| Base de lenguaje v1.0.1 (Fase 1 freeze) | SI | SI | SI | SI | SI | SI | SI | SI | SI |
| Modelo de toolchains v1.0.1 (Fase 2 freeze) | SI | SI | SI | SI | SI | SI | SI | SI | SI |
| Runtime + stdlib v1.0.1 (Fase 3 freeze) | SI | SI | SI | SI | SI | SI | SI | SI | SI |
| Backend soberano + ABI reales v1.0.1 (Fase 4 freeze) | SI | SI | SI | SI | SI | SI | SI | SI | SI |
| Toolchain completa v1.0.1 (Fase 5 freeze) | SI | SI | SI | SI | SI | SI | SI | SI | SI |
| Calidad extrema v1.0.1 (Fase 6 freeze) | SI | SI | SI | SI | SI | SI | SI | SI | SI |
| Ecosistema de paquetes y builds v1.0.1 (Fase 7 freeze) | SI | SI | SI | SI | SI | SI | SI | SI | SI |
| DX y observabilidad v1.0.1 (Fase 8 freeze) | SI | SI | SI | SI | SI | SI | SI | SI | SI |
| Gobernanza y ciclo de releases v1.0.1 (Fase 9 freeze) | SI | SI | SI | SI | SI | SI | SI | SI | SI |
| Documentación de versión (F22) | N/A | N/A | N/A | N/A | N/A | N/A | N/A | SI | SI |
| Compatibilidad 1.x (F23) | N/A | N/A | N/A | N/A | N/A | N/A | N/A | SI | SI |
| Release engineering (F24) | N/A | N/A | N/A | N/A | SI | SI | SI | SI | SI |

Referencia de evidencia F13:

- `docs/spec/security/security_operational_contracts_v1_0.md`

Referencia de evidencia F14:

- `docs/spec/network/network_operational_contracts_v1_0.md`

Referencia de evidencia F15:

- `docs/spec/system/system_operational_contracts_v1_0.md`

Referencia de evidencia F16:

- `docs/spec/tooling/tooling_operational_contracts_v1_0.md`

Referencia de evidencia F17:

- `docs/spec/compiler/compiler_operational_contracts_v1_0.md`

Referencia de evidencia F18:

- `docs/spec/kernel/kernel_operational_contracts_v1_0.md`

Referencia de evidencia F19:

- `docs/spec/ai/ai_operational_contracts_v1_0.md`

Referencia de evidencia F20:

- `docs/spec/backend/backend_material_defensible_v1_0.md`

Referencia de evidencia Fase 1 (freeze de lenguaje v1.0.1):

- `docs/spec/release/v1_0_1_language_spec_immutable.md`
- `docs/spec/release/v1_0_1_compatibility_spec.md`
- `docs/spec/release/v1_0_1_language_freeze_contract.json`

Referencia de evidencia Fase 2 (modelo de toolchains v1.0.1):

- `docs/spec/release/v1_0_1_toolchain_model_spec.md`
- `docs/spec/release/v1_0_1_toolchain_compatibility_spec.md`
- `docs/spec/release/v1_0_1_toolchain_model_contract.json`

Referencia de evidencia Fase 3 (runtime + stdlib v1.0.1):

- `docs/spec/release/v1_0_1_runtime_stdlib_spec.md`
- `docs/spec/release/v1_0_1_runtime_stdlib_compatibility_spec.md`
- `docs/spec/release/v1_0_1_runtime_stdlib_contract.json`

Referencia de evidencia Fase 4 (backend soberano + ABI v1.0.1):

- `docs/spec/release/v1_0_1_backend_sovereign_spec.md`
- `docs/spec/release/v1_0_1_backend_sovereign_compatibility_spec.md`
- `docs/spec/release/v1_0_1_backend_sovereign_contract.json`

Referencia de evidencia Fase 5 (toolchain completa v1.0.1):

- `docs/spec/release/v1_0_1_toolchain_complete_spec.md`
- `docs/spec/release/v1_0_1_toolchain_complete_compatibility_spec.md`
- `docs/spec/release/v1_0_1_toolchain_complete_contract.json`

Referencia de evidencia Fase 6 (calidad extrema v1.0.1):

- `docs/spec/release/v1_0_1_quality_extreme_spec.md`
- `docs/spec/release/v1_0_1_quality_extreme_compatibility_spec.md`
- `docs/spec/release/v1_0_1_quality_extreme_contract.json`

Referencia de evidencia Fase 7 (ecosistema de paquetes y builds v1.0.1):

- `docs/spec/release/v1_0_1_package_ecosystem_spec.md`
- `docs/spec/release/v1_0_1_package_ecosystem_compatibility_spec.md`
- `docs/spec/release/v1_0_1_package_ecosystem_contract.json`

Referencia de evidencia Fase 8 (DX y observabilidad v1.0.1):

- `docs/spec/release/v1_0_1_dx_observability_spec.md`
- `docs/spec/release/v1_0_1_dx_observability_compatibility_spec.md`
- `docs/spec/release/v1_0_1_dx_observability_contract.json`

Referencia de evidencia Fase 9 (gobernanza y ciclo de releases v1.0.1):

- `docs/spec/release/v1_0_1_governance_release_spec.md`
- `docs/spec/release/v1_0_1_governance_release_compatibility_spec.md`
- `docs/spec/release/v1_0_1_governance_release_contract.json`

Referencia de evidencia F21:

- `docs/spec/release/f21_acceptance_gate_v1_0.md`

Referencia de evidencia F22:

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

Referencia de evidencia F23:

- `docs/spec/release/v1_0_compatibility_policy_1x.md`
- `docs/spec/release/v1_0_compatibility_contract.json`
- `docs/spec/release/f23_compatibility_freeze_v1_0.md`

Referencia de evidencia F24:

- `docs/spec/release/f24_release_engineering_v1_0.md`
- `docs/spec/release/v1_0_changelog.md`
- `docs/spec/release/v1_0_release_notes.md`
- `docs/spec/release/v1_0_release_artifacts_manifest.json`
- `tools/release/repro_build_v1_0.sh`
- `tools/release/smoke_v1_0.sh`

## 3. Regla de promoción a `SI`

Una fila solo sube a `SI` cuando:

1. todas sus columnas aplicables están en `SI`;
2. no hay fallback stringly en zonas tipadas donde el base exige estructura;
3. existe evidencia reproducible (`cargo test --workspace -- --nocapture` + comandos de ejecución de fixtures de la fase).

## 4. Comando de verificación mínima global

```bash
cargo test --workspace -- --nocapture
```

```bash
cargo run -p ncc-cli -- check <fixture_fase.n>
cargo run -p ncc-cli -- run <fixture_fase.n>
```
