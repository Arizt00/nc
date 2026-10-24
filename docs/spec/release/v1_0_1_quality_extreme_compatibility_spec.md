# nC Quality Extreme Compatibility Spec v1.0.1

Estado: `frozen`  
Serie objetivo: `1.x`

## Objetivo

Fijar compatibilidad de hardening/QA/fuzz/performance en Fase 6 para toda la serie `1.x`.

## Regla principal

En `1.x` no se permiten cambios breaking en:

- contrato de fuzz determinista parser/sema/runtime sin panic;
- contrato de benchmark smoke por presupuesto amplio;
- diagnósticos de seguridad canónicos de regresión (`SEC047`, enforcement runtime);
- reproducibilidad material de salidas bajo mismo input/profile.

## Cambios permitidos en `1.x`

- ampliar corpus de fuzz y cobertura;
- endurecer validaciones manteniendo compatibilidad;
- mejorar performance sin alterar semántica observable;
- mejorar documentación y tooling de QA.

## Cambios prohibidos en `1.x`

- introducir pánicos en rutas de parser/sema/runtime ante entradas hostiles;
- retirar suites críticas de fuzz/benchmark/regresión;
- cambiar diagnósticos canónicos de seguridad sin RFC mayor.

## Evidencia machine-readable

- `docs/spec/release/v1_0_1_quality_extreme_contract.json`

## Gate obligatorio

- `release_phase6_quality_extreme_gate_tests`
