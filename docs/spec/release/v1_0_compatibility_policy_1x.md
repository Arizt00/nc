# nC v1.0.0 — Política oficial de compatibilidad 1.x (F23)

Fecha de congelación: 2026-04-30
Estado: `frozen`

## 1. Objetivo

Fijar una política estricta de compatibilidad para toda la serie `1.x`.
Esta política aplica a la línea activa del compilador y al base de lenguaje publicado para v1.0.0.

## 2. Regla SemVer obligatoria

- `MAJOR` (`2.0.0+`): cambios incompatibles de superficie/semántica.
- `MINOR` (`1.y.0`): capacidades nuevas compatibles hacia atrás.
- `PATCH` (`1.0.z`): correcciones compatibles, sin cambio de contrato observable.

## 3. Qué rompe compatibilidad (solo permitido en MAJOR)

Se considera **breaking change** (prohibido en `1.x`) cualquier cambio que altere:

- gramática o parse aceptado de superficies estables;
- contratos tipados AST/sema/IR de fases cerradas;
- diagnósticos canónicos `FAIL` (texto/criterio) sin migración mayor;
- comportamiento runtime/lowering/backend observable en fixtures canónicos;
- precedencia/sentido de operadores ya congelados;
- reglas de continuidad modal `[@]`/`[@machine]`/`primitive[@]` y modos F13..F22 cerrados.

## 4. Qué sí puede entrar en 1.x sin romper

Permitido en `1.x` si preserva contrato canónico:

- bugfixes que corrigen desalineación respecto al base vigente;
- hardening interno sin alterar semántica observable estable;
- nuevos imports/familias en estado explícitamente compatible;
- mejoras de rendimiento sin cambiar salida semántica/material del base;
- documentación y cobertura de tests.

## 5. Política PATCH (`1.0.z`)

Un patch release **no puede**:

- introducir nuevos requisitos de código fuente para programas válidos en `1.0.0`;
- cambiar diagnósticos canónicos cerrados salvo corrección de bug documentada y gateada;
- modificar la matriz oficial cerrada (`SI`) a estados inferiores.

Un patch release **sí puede**:

- arreglar bugs de implementación;
- reforzar validaciones sin romper contratos estables;
- añadir pruebas y documentación.

## 6. Política MINOR (`1.y.0`)

Un minor release puede ampliar superficie bajo estas condiciones:

- extensión aditiva compatible;
- sin deprecación destructiva inmediata;
- con actualización de matriz/evidencia/gates.

Toda extensión minor debe mantener `check`/`run` de fixtures cerrados de `1.0.0`.

## 7. Superficies congeladas tras v1.0.0

Quedan congeladas en `1.x` (sin breaking):

- F1 `[@]`
- F2 `[@machine]`
- F3 `primitive[@]`
- F4 linking soberano
- F5 memoria/punteros/regiones/alias
- F6 runtime profundo base
- F7 inspect
- F11 base ABI/lowering/backend
- F12, F13, F14, F15, F16, F17, F18, F19, F20, F21, F22, F23, F24

## 8. Cambios explícitamente prohibidos en 1.x

- degradar rutas tipadas a parsing textual crudo;
- romper reglas canónicas de terminación/alcanzabilidad de `[@machine]`;
- aflojar contratos de seguridad/network/system/kernel/ai cerrados;
- alterar contratos de compatibilidad perfil/artefacto del backend sin versión mayor;
- retirar tooling oficial (`ncc`, `nfmt`, `nlint`, `nbuild`, `npkg`) del flujo base.

## 9. Canal experimental en 1.x

Solo se permiten experimentos si:

- están marcados explícitamente como experimentales;
- no alteran el comportamiento estable por defecto;
- pueden desactivarse sin afectar el base estable.

## 10. Gobernanza de cambios

Todo cambio potencialmente sensible en 1.x requiere:

1. propuesta con impacto en compatibilidad;
2. actualización de documentación canónica;
3. gate de tests (F21/F22/F23/F24 + transversal v1.0.1) en verde;
4. validación completa `cargo test --workspace -- --nocapture`.

Además, para cierres de superficie congelada de lenguaje aplica RFC obligatorio con template:

- `docs/spec/release/rfc/RFC_TEMPLATE.md`

## 11. Evidencia y contrato machine-readable

Contrato complementario:

- `docs/spec/release/v1_0_compatibility_contract.json`
- `docs/spec/release/v1_0_1_language_freeze_contract.json`
- `docs/spec/release/v1_0_1_toolchain_model_contract.json`
- `docs/spec/release/v1_0_1_runtime_stdlib_contract.json`
- `docs/spec/release/v1_0_1_backend_sovereign_contract.json`
- `docs/spec/release/v1_0_1_toolchain_complete_contract.json`
- `docs/spec/release/v1_0_1_quality_extreme_contract.json`
- `docs/spec/release/v1_0_1_package_ecosystem_contract.json`
- `docs/spec/release/v1_0_1_dx_observability_contract.json`
- `docs/spec/release/v1_0_1_governance_release_contract.json`

Este archivo permite validación automática de invariantes de compatibilidad en tests de gate F23, Fase 1, Fase 2, Fase 3, Fase 4, Fase 5, Fase 6, Fase 7, Fase 8 y Fase 9.

## 12. Release train oficial 1.x

El release train oficial para `1.x` usa canales `stable, beta y nightly`.

- `stable`: release de confianza para producción.
- `beta`: pre-release de validación antes de `stable`.
- `nightly`: integración continua y validación temprana.

Todos los canales siguen la regla de compatibilidad 1.x: sin breaking changes fuera de `MAJOR`.

## 13. Política de deprecaciones

- toda deprecación debe anunciarse con warning + release notes + changelog;
- debe indicar versión objetivo de remoción;
- mínimo de espera antes de remoción: 2 releases `stable`;
- deprecaciones con impacto breaking requieren RFC aprobado.

## 14. Política LTS

- la serie `1.x` puede designar cortes LTS sobre `stable`;
- ventana objetivo de soporte LTS: 24 meses;
- backports LTS: seguridad, bugs críticos y data-loss.

## 15. Soporte empresarial

- tracks soportados: `stable` y `lts`;
- SLA objetivo de respuesta de seguridad: 24h;
- objetivo de hotfix crítico: 72h;
- acuerdos empresariales no pueden contradecir el base técnico de compatibilidad.

## 16. Regla de predictibilidad para usuarios

Toda publicación oficial debe dejar explícito qué rompe, cuándo y por qué.
Esto aplica a release notes, changelog y documentos de compatibilidad asociados.

## 17. Endurecimiento operativo de releases

- promoción `nightly/beta/stable` automatizada y trazable en CI/CD;
- warnings de deprecación de compilación con códigos estables `NCC-DEPRECATION-*`;
- bundle de integridad por release (checksums/firma/SBOM/attestation);
- runbook LTS/enterprise con SLA validable por script.
