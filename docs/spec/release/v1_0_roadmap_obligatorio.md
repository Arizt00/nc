# nC v1.0.0 — Roadmap obligatorio

Fecha de corte inicial: 2026-04-30

Este documento fija la ejecución obligatoria para v1.0.0 y complementa `_archive/deprecated/context_legacy/roadmap/ROADMAP_MASTER.md` (sección 25).

## 1. Condición de entrada

Se toma como base cerrada en línea activa:

- F0 fundación
- F1 `[@]`
- F2 `[@machine]`
- F3 `primitive[@]`
- F4 linking soberano
- F5 memoria/punteros/regiones/alias
- F6 runtime profundo base
- F7 inspect
- F11 ABI/lowering/backend base
- F12 estabilización v0.1

## 2. Regla de orden obligatorio

Primero se ejecuta el bloque inferior transversal y después los tramos F13..F24.

### 2.1 Bloque inferior transversal (obligatorio antes de tramos)

1. operadores tipados end-to-end y precedencia congelada;
2. expansión de ISA de `[@machine]`;
3. profundización radical de `primitive[@]`;
4. multi-target más ambicioso;
5. tooling más sofisticado;
6. nuevas familias de `use/import`;
7. optimizaciones avanzadas;
8. ecosistema IDE profundo.

Anclaje documental inicial de operadores:

- `docs/spec/root/operators_v1_0_closure_matrix.md`

### 2.2 Estado de ejecución (checkpoint 2026-04-30)

- `SI` bloque 2.1.1 (operadores tipados end-to-end y precedencia congelada).
- `SI` bloque 2.1.2 (expansión ISA `[@machine]` con continuidad tipada completa).
- `SI` bloque 2.1.3 (profundización radical `primitive[@]` con materialización backend/object de contratos tipados).
- `SI` bloque 2.1.4 (multi-target ambicioso con coherencia de formatos activos).
- `SI` bloque 2.1.5 (tooling sofisticado con comandos ecosistema tipados `nfmt`/`nlint`/`nbuild`/`npkg`).
- `SI` F13 (security operativo completo con enforcement runtime + inspect + matriz de fixtures canónicos).
- `SI` F14 (network ejecutable completo con flujo/errores runtime canónicos + matriz de fixtures).
- `SI` F15 (system operativo completo con lifecycle/health/recovery observables + matriz de fixtures canónicos).
- `SI` F16 (tooling ecosistema operativo con `nfmt`/`nlint`/`nbuild`/`npkg`, contrato tipado `tooling.build/lint/fmt/pkg` y fixtures canónicos).
- `SI` F17 (compiler operativo completo con baseline/flow tipado `parse/check/materialize`, estado runtime observable y matriz de fixtures canónicos).
- `SI` F18 (kernel mínimo serio con baseline/flow tipado `register/open/next/quarantine/panic`, inspect runtime `kernel.*` y matriz de fixtures canónicos).
- `SI` F19 (ai operativo completo con contratos tipados `session/model/agent/chat/memory`, inspect runtime `ai.*` y matriz de fixtures canónicos).
- `SI` F20 (backend material defendible con contrato tipado de compatibilidad artefacto/ABI, verificación de salida material no vacía y reproducibilidad ELF64).
- `SI` F21 (matriz oficial de aceptación v1.0.0 con gate ejecutable doc<->implementación y cobertura `OK/FAIL` por modo activo).
- `SI` F22 (documentación mayor completa: spec maestra, roadmap coherente, acta binaria, guías de uso/modos/build/tooling, handoff y tablas oficiales).
- `SI` F23 (política de compatibilidad 1.x congelada en contrato formal + contrato machine-readable + gate ejecutable).
- `SI` Fase 1 freeze de lenguaje v1.0.1 (`@`, `>>`, `!>`, `main`, `print`, modos) con contrato inmutable + compatibilidad/RFC.
- `SI` Fase 2 modelo de toolchains v1.0.1 (canales `stable/beta/nightly`, ids `<channel>-<target>`, componentes y perfiles de instalación) con contrato inmutable + gate dedicado.
- `SI` Fase 3 runtime + stdlib v1.0.1 (runtime de producción `hosted/portable/freestanding`, API base `io/mem/collections/net/time/trace/inspect`) con contrato inmutable + gate dedicado.
- `SI` Fase 4 backend soberano + ABI reales v1.0.1 (matriz target tiered, contratos de lowering por target, y emisiones reproducibles ELF/COFF/Mach-O/WASM) con contrato inmutable + gate dedicado.
- `SI` Fase 5 toolchain completa v1.0.1 (CLI UX estable, help canónico, errores accionables y operación fuera del repo en instalación limpia) con contrato inmutable + gate dedicado.
- `SI` Fase 6 calidad extrema v1.0.1 (hardening QA/fuzz/seguridad/performance con gate de no-panic y budgets de benchmark smoke) con contrato inmutable + gate dedicado.
- `SI` Fase 7 ecosistema de paquetes y builds v1.0.1 (resolver de dependencias, semver/registry, lockfile determinista y verificación de drift) con contrato inmutable + gate dedicado.
- `SI` Fase 8 DX y observabilidad v1.0.1 (códigos de error estables, sugerencias automáticas y perfiles de ejecución `dx-profile`) con contrato inmutable + gate dedicado.
- `SI` Fase 9 gobernanza y ciclo de releases v1.0.1 (release train `stable/beta/nightly`, RFC/deprecaciones/LTS/soporte empresarial, política de retrocompatibilidad y predictibilidad) con contrato inmutable + gate dedicado.
- Evidencia activa:
  - parser: soporte dual tipado en `ref main`/`skkMain` para `->>` (`@fn(...)` y ruta directa).
  - sema: validación de autoridad/continuidad para rutas directas en `->>`.
  - runtime: ejecución de `FnRef.path` completa en main entry.
  - tests: `cargo test --workspace` en verde.
  - machine ISA expandida: `mul/adc/sbb/sar/xchg/neg.bin` en AST/parser/sema/IR/backend + tests de lowering y emisión nativa.
  - primitive profundo: backend `primitive` ahora materializa contratos tipados de `capability/bitfield/operation/signal/port/signature/entry` en `.primitive` y `.meta.*` con pruebas en `ncc-ir` y `ncc-cli`.
  - multi-target activo: `x86_64` + `aarch64` en ABI/lowering/ELF64 tipado; `emit-coff` y `emit-macho` con ruta `objcopy` y fallback `clang` tipado, cubiertos por tests CLI.
  - kernel operativo: parser/AST/sema/IR/runtime cerrados para `[@kernel]`, contratos inspect kernel y fixtures `examples/kernel_at/*` enlazados con pruebas en `ncc-cli`.
  - ai operativo: parser/AST/sema/IR/runtime cerrados para `[@ai]`, contratos inspect ai y fixtures `examples/ai_at/*` enlazados con pruebas en `ncc-cli`.
  - backend material defendible: endurecimiento CLI para contratos perfil/artefacto (`emit-elf`/`build-exe`/`emit-macho`), verificación obligatoria de salida material y tests de determinismo/linker en `backend_emit_cli_tests`.
  - F21 aceptación: gate dedicado `release_f21_acceptance_gate_tests` valida parser/sema/lowering/backend/runtime y diagnósticos `FAIL` canónicos por modo; además bloquea deriva del documento de matriz oficial.
  - F22 documentación: bundle canónico de release en `docs/spec/release/v1_0_*` con gate dedicado `release_f22_documentation_gate_tests`.
  - F23 compatibilidad: política 1.x formal en `v1_0_compatibility_policy_1x.md` + `v1_0_compatibility_contract.json`, validadas por `release_f23_compatibility_gate_tests`.
  - Fase 1 freeze de lenguaje: `v1_0_1_language_spec_immutable.md` + `v1_0_1_compatibility_spec.md` + `v1_0_1_language_freeze_contract.json`, validadas por `release_phase1_language_base_gate_tests`.
  - Fase 2 modelo de toolchains: `v1_0_1_toolchain_model_spec.md` + `v1_0_1_toolchain_compatibility_spec.md` + `v1_0_1_toolchain_model_contract.json`, validadas por `release_phase2_toolchain_model_gate_tests`.
  - Fase 3 runtime+stdlib: `v1_0_1_runtime_stdlib_spec.md` + `v1_0_1_runtime_stdlib_compatibility_spec.md` + `v1_0_1_runtime_stdlib_contract.json`, validadas por `release_phase3_runtime_stdlib_gate_tests`.
  - Fase 4 backend soberano+ABI: `v1_0_1_backend_sovereign_spec.md` + `v1_0_1_backend_sovereign_compatibility_spec.md` + `v1_0_1_backend_sovereign_contract.json`, validadas por `release_phase4_backend_sovereign_gate_tests`.
  - Fase 5 toolchain completa: `v1_0_1_toolchain_complete_spec.md` + `v1_0_1_toolchain_complete_compatibility_spec.md` + `v1_0_1_toolchain_complete_contract.json`, validadas por `release_phase5_toolchain_complete_gate_tests`.
  - Fase 6 calidad extrema: `v1_0_1_quality_extreme_spec.md` + `v1_0_1_quality_extreme_compatibility_spec.md` + `v1_0_1_quality_extreme_contract.json`, validadas por `release_phase6_quality_extreme_gate_tests`.
  - Fase 7 ecosistema de paquetes/builds: `v1_0_1_package_ecosystem_spec.md` + `v1_0_1_package_ecosystem_compatibility_spec.md` + `v1_0_1_package_ecosystem_contract.json`, validadas por `release_phase7_package_ecosystem_gate_tests`.
  - Fase 8 DX/observabilidad: `v1_0_1_dx_observability_spec.md` + `v1_0_1_dx_observability_compatibility_spec.md` + `v1_0_1_dx_observability_contract.json`, validadas por `release_phase8_dx_observability_gate_tests`.
  - Fase 9 gobernanza/release-cycle: `v1_0_1_governance_release_spec.md` + `v1_0_1_governance_release_compatibility_spec.md` + `v1_0_1_governance_release_contract.json`, validadas por `release_phase9_governance_release_gate_tests`.

## 3. Tramos obligatorios hacia v1.0.0

### Tramo 1

- F13 Security operativo completo. (`SI` — cerrado en línea activa 2026-04-30)
- F14 Network ejecutable completo. (`SI` — cerrado en línea activa 2026-04-30)

### Tramo 2

- F15 System operativo. (`SI` — cerrado en línea activa 2026-04-30)
- F16 Tooling real de ecosistema (`ncc`, `nfmt`, `nlint`, `nbuild`, `npkg`). (`SI` — cerrado en línea activa 2026-04-30)
- F17 `[@compiler]` con frontera clara. (`SI` — cerrado en línea activa 2026-04-30)

### Tramo 3

- F18 Kernel mínimo serio. (`SI` — cerrado en línea activa 2026-04-30)
- F19 decisión oficial de alcance de `[@ai]`. (`SI` — cerrado en línea activa 2026-04-30)

### Tramo 4

- F20 Backend material defendible. (`SI` — cerrado en línea activa 2026-04-30)
- F21 Matriz oficial de aceptación 1.0.0. (`SI` — cerrado en línea activa 2026-04-30)
- F22 Documentación mayor. (`SI` — cerrado en línea activa 2026-04-30)
- F23 Congelación de compatibilidad. (`SI` — cerrado en línea activa 2026-04-30)
- F24 Release engineering. (`SI` — cerrado en línea activa 2026-04-30)

## 4. Reglas de cierre por fase

Una fase solo puede marcarse `SI` cuando tiene continuidad real por capa:

- parser
- AST
- sema
- IR
- runtime/lowering/backend (según aplique)
- fixtures OK
- fixtures FAIL canónicos
- documentación final

No se acepta como cierre:

- check OK con run faltante en superficie oficial;
- semántica aceptada pero sin contrato operativo observable;
- diagnósticos vagos no canónicos.

## 5. Criterio binario de salida v1.0.0

No hay release `v1.0.0` si falta cualquiera de:

1. bloque inferior transversal completo;
2. cierre binario de F13..F24;
3. matriz oficial de aceptación;
4. documentación maestra de versión;
5. política de compatibilidad;
6. release reproducible en entorno limpio + WSL.

## 6. Regla de compatibilidad

Toda ampliación durante v1.0.0 debe:

- preservar continuidad tipada interna;
- no degradar estructuras tipadas existentes a parsing textual crudo;
- mantener trazabilidad en docs/base/tests.
