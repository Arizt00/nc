# nC Quality Extreme Spec v1.0.1

Estado: `frozen`  
Serie: `1.x`  
Fase: `phase6_quality_extreme`

## Objetivo

Cerrar calidad extrema de producción para la toolchain y runtime de nC:

- hardening operativo de extremo a extremo;
- fuzzing parser/sema/runtime;
- benchmarks de humo con presupuesto estable;
- regresión de seguridad canónica;
- regression gates de release sin pánicos.

## Alcance obligatorio

1. `fuzzing parser/sema/runtime` con mutación determinista y tolerancia a entradas dañadas sin panic.
2. `benchmarks` de humo para lex/parse/sema/backend con presupuesto amplio y estable.
3. `seguridad` en regresión (diagnósticos canónicos + enforcement runtime).
4. `reproducibilidad` material en emisiones críticas bajo mismo input/profile.
5. `cero regresiones` en gates críticos de fases 1..5.

## Suites operativas de Fase 6

- `release_phase6_quality_extreme_gate_tests::phase6_parser_sema_runtime_mutation_fuzz_does_not_panic`
- `release_phase6_quality_extreme_gate_tests::phase6_benchmark_smoke_and_reproducible_outputs_stay_stable`
- `release_phase6_quality_extreme_gate_tests::phase6_security_and_runtime_regression_contracts_stay_hardened`

Scripts de soporte:

- `tools/release/qa_phase6_v1_0_1.sh`
- `tools/release/fuzz_phase6_v1_0_1.sh`
- `tools/release/bench_phase6_v1_0_1.sh`

## DoD

Fase 6 se considera cerrada solo si:

1. spec + compat + contract están en `frozen`;
2. gate `release_phase6_quality_extreme_gate_tests` está en verde;
3. no hay pánicos en fuzz determinista parser/sema/runtime;
4. benchmarks de humo quedan dentro de presupuesto estable;
5. releases reproducibles y gates críticos quedan sin regresiones.

## Gate obligatorio

- `release_phase6_quality_extreme_gate_tests`
