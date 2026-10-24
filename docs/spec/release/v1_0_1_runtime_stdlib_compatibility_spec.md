# nC Runtime + Stdlib Compatibility Spec v1.0.1

Estado: `frozen`  
Serie: `1.x`

## Objetivo

Fijar reglas de compatibilidad para runtime + stdlib base cerrados en fase 3.

## Regla principal

No se permiten cambios breaking en `1.x` sobre:

- aridad de imports runtime oficiales;
- tipos de retorno de imports runtime oficiales;
- alias canónicos `nC.runtime.*`;
- semántica observable de `io/mem/collections/net/time/trace/inspect`.
- catálogo congelado de `runtime_families` (90 entradas).

## Qué es breaking

Se considera breaking (prohibido sin RFC):

- eliminar funciones de módulos runtime congelados;
- cambiar retorno de `runtime.time.now/monotonic/sleep`;
- cambiar retorno de `memory.heap.manager.*`;
- romper wildcard `use nC.runtime.*` para módulos ya congelados;
- alterar diagnóstico canónico de incompatibilidad de firma import runtime.
- remover o renombrar familias en `runtime_families`.

## Qué es aditivo permitido

Se permite (sin romper `1.x`):

- agregar nuevos módulos fuera del set congelado;
- agregar nuevas funciones opcionales en módulos nuevos;
- agregar metadata de inspección sin romper formatos existentes.
- agregar metadata no-breaking en cada familia runtime congelada.

## Gobernanza

- `no_breaking_without_rfc = true`
- RFC obligatorio para cualquier cambio con riesgo de incompatibilidad.

Referencia:

- `docs/spec/release/rfc/RFC_TEMPLATE.md`

## Contrato machine-readable

- `docs/spec/release/v1_0_1_runtime_stdlib_contract.json`

## Gate obligatorio

- `release_phase3_runtime_stdlib_gate_tests`
