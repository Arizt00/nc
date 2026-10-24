# nC Language Spec v1.0.1 (Immutable)

Fecha de congelación: 2026-05-02
Estado: `frozen` / `immutable`
Ámbito: parser, sema, IR, runtime y CLI en la serie `1.x`.

## 1. Propósito

Este documento congela el base de lenguaje `v1.0.1` para las superficies nucleares:

- operador referencial `@`;
- operador de proyección `>>`;
- operador de chequeo estricto `!>`;
- familia `main` (`to main`, `to main@<profile>`, `ref main`, `skkMain`/`skk main`);
- familia `print` y políticas de render;
- modos oficiales del lenguaje.

Regla normativa:

- En `1.x` no se permiten cambios breaking sobre esta superficie.
- Cualquier excepción requiere RFC aprobado y salto de `MAJOR`.

## 2. Jerarquía canónica

Para conflictos de interpretación, prevalece este orden:

1. `docs/spec/release/v1_0_1_language_spec_immutable.md`
2. `docs/spec/release/v1_0_1_compatibility_spec.md`
3. `docs/spec/release/v1_0_1_language_freeze_contract.json`
4. specs por dominio (`docs/spec/root/*`, `docs/spec/runtime/*`, `docs/spec/backend/*`)
5. matriz y roadmap de release (`docs/spec/release/v1_0_acceptance_matrix.md`, `docs/spec/release/v1_0_roadmap_obligatorio.md`)

## 3. Superficie congelada

### 3.1 `@` (referencial)

Contrato congelado:

- `@` identifica entidad contextual resoluble (símbolo/ruta/función/probe según forma);
- resolución ambigua falla con diagnóstico canónico;
- `@` no implica puntero material implícito ni `addr` automático;
- la autoridad modal y la visibilidad aplican siempre.

Referencia normativa existente:

- `docs/spec/root/at_surface_official_v0_3.md`
- `docs/spec/root/linking_resolution_precedence_v0_1.md`

### 3.2 `>>` (proyección)

Contrato congelado:

- `@dest >> Tipo : expr;` proyecta bajo contrato tipado;
- destino debe existir y ser escribible;
- no se permite narrowing implícito destructivo;
- se respeta `strictly`, nullabilidad y compatibilidad tipada.

Referencia normativa existente:

- `docs/spec/root/operators_surface_official_v0_1.md`
- `docs/spec/root/pointer_compatibility_official_matrix_v0_1.md`

### 3.3 `!>` (strict-check)

Contrato congelado:

- valida compatibilidad/refinamiento tipado de referencia resuelta;
- no resuelve símbolos por sí mismo;
- cast/cross-domain fuera de matriz canónica falla con diagnóstico.

Referencia normativa existente:

- `docs/spec/root/operators_surface_official_v0_1.md`
- `docs/spec/root/pointer_compatibility_official_matrix_v0_1.md`

### 3.4 Familia `main`

Formas activas congeladas:

- `to main { ... }`
- `to main@<profile> { ... }`
- `ref main ->> @fn(...)`
- `ref main ->> ruta.directa`
- `skkMain ->> ...`
- alias compatible: `skk main ->> ...`

La selección de entrada, límites estructurales y contrato de retorno quedan congelados.

Referencia normativa existente:

- `docs/spec/root/main_family_contract_v2_0_0.md`

### 3.5 Familia `print` + render modes

Builtins congelados:

- `print`, `println`, `print.sp`, `print.nl`
- `print.err`, `print.wm`
- `print.trc`, `print.dbg`, `print.sta`
- `print.inspect`, `print.mem`, `print.st`, `print.state`

Render modes congelados:

- `human`
- `debug`
- `inspect`
- `stable`

Canal y política de newline por builtin quedan congelados.

Referencia normativa existente:

- `docs/spec/runtime/console_print_contract_v2.md`
- `docs/spec/runtime/console_builtin_matrix_v2_0_0.md`

### 3.6 Modos oficiales

La continuidad tipada y operativa de estos modos queda congelada en `1.x`:

- `[@]`
- `[@machine]`
- `primitive[@]`
- `[@inspect]`
- `[@security]`
- `[@network]`
- `[@system]`
- `[@tooling]`
- `[@compiler]`
- `[@kernel]`
- `[@ai]`

Referencia normativa existente:

- `docs/spec/release/v1_0_acceptance_matrix.md`

## 4. Invariantes parser/sema/IR

Invariantes obligatorios de `v1.0.1`:

- parser no degrada contratos tipados a interpretación stringly;
- sema mantiene diagnósticos canónicos y autoridad modal;
- IR preserva significado semántico (no altera contrato de superficie);
- runtime/backend no reinterpretan semántica en contradicción con sema.

## 5. Diagnósticos canónicos congelados

Se congela la familia de diagnósticos para:

- resolución/ambigüedad de `main`;
- incompatibilidad estricta de `!>`;
- rutas inválidas de `print.*`;
- fallos de autoridad/resolución en referencias `@...`.

Cambios de texto o criterio en esta familia son breaking en `1.x`.

## 6. Evidencia ejecutable (gate)

Gate obligatorio de esta fase:

- `release_phase1_language_base_gate_tests`

Este gate valida:

- existencia e integridad de artefactos de freeze;
- presencia de reglas de compatibilidad sin breaking en `1.x`;
- coherencia parser/sema/IR en fixtures canónicos de superficie;
- estabilidad de diagnósticos negativos clave.

## 7. Estado de cierre de Fase 1

Fase 1 (`Base del lenguaje congelado`) se considera cerrada cuando:

1. este documento está en estado `frozen`;
2. `v1_0_1_compatibility_spec.md` está en estado `frozen`;
3. el contrato machine-readable está en estado `frozen`;
4. el gate `release_phase1_language_base_gate_tests` pasa en verde;
5. parser/sema/IR mantienen alineación en los fixtures de referencia.

