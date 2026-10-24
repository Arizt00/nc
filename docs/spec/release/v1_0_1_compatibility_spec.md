# nC Compatibility Spec v1.0.1

Fecha: 2026-05-02
Estado: `frozen`
Serie aplicable: `1.x`

## 1. Objetivo

Definir el contrato de compatibilidad obligatorio para mantener estabilidad de lenguaje en `1.x`.

Regla principal:

- **No changes breaking without RFC**.
- En `1.x`, un cambio breaking está prohibido; solo puede entrar en `2.0.0+` con RFC aprobado.

## 2. Política SemVer obligatoria

- `MAJOR` (`2.0.0+`): cambios incompatibles permitidos.
- `MINOR` (`1.y.0`): cambios aditivos compatibles.
- `PATCH` (`1.0.z`): correcciones compatibles, sin cambio de contrato observable.

## 3. Qué se considera breaking en `1.x`

Se considera breaking, y por tanto prohibido en `1.x`, cualquier cambio que altere:

- gramática aceptada de superficies congeladas (`@`, `>>`, `!>`, `main`, `print`, modos);
- contratos tipados parser/AST/sema/IR;
- diagnósticos canónicos de fixtures negativos;
- precedencia/semántica de operadores congelados;
- selección y contrato de retorno de la familia `main`;
- contrato de salida/builtin/canal/render de la familia `print`.

## 4. Qué sí puede entrar en `1.x`

Permitido en `1.x` si es compatible:

- bugfixes que corrigen una desviación del base congelado;
- endurecimiento interno sin alterar salida observable canónica;
- mejoras de performance sin cambiar semántica;
- ampliaciones aditivas explícitamente marcadas como compatibles.

## 5. Gobernanza y RFC obligatorio

No se acepta ningún cambio con impacto potencial de compatibilidad sin RFC.

Proceso mínimo:

1. crear RFC a partir de `docs/spec/release/rfc/RFC_TEMPLATE.md`;
2. completar sección de impacto en compatibilidad;
3. clasificar el cambio (`major/minor/patch`);
4. registrar migración y riesgos;
5. actualizar gates y evidencias;
6. aprobar RFC antes de merge.

Regla dura:

- Si cambia una superficie congelada y no existe RFC aprobado, el cambio se considera inválido.

## 6. Contrato machine-readable

Contrato vinculante:

- `docs/spec/release/v1_0_1_language_freeze_contract.json`

Campos críticos:

- `state = frozen`
- `series = 1.x`
- `no_breaking_without_rfc = true`
- `required_gates` incluye `release_phase1_language_base_gate_tests`

## 7. Criterio de cumplimiento de Fase 1

Fase 1 queda en cumplimiento si:

1. gate de freeze en verde (`release_phase1_language_base_gate_tests`);
2. suite parser/sema/IR en verde para fixtures canónicos;
3. no hay cambios breaking abiertos sin RFC;
4. los contratos congelados permanecen en estado `frozen`.

