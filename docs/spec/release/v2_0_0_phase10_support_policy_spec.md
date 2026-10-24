# nC Phase10 Support Policy Spec v2.0.0

Estado: `frozen`  
Fase: `phase10_p17_support_policy`

## Objetivo

Cerrar F10-P17 con política institucional de soporte para `stable|beta|nightly` y ventana LTS empresarial explícita:

1. `stable`: soporte productivo completo;
2. `beta`: preview con migración documentada;
3. `nightly`: experimental sin SLA;
4. ventana LTS formal para entornos enterprise.

## Política canónica por canal

1. `stable`
- DEBE considerarse canal productivo.
- DEBE recibir soporte completo de toolchain y runtime en targets Tier 1.
- DEBE publicarse con notas de compatibilidad y riesgos residuales.

2. `beta`
- DEBE considerarse canal preview pre-productivo.
- DEBE incluir guía de migración documentada antes de promoción a `stable`.
- PUEDE contener cambios aún no consolidados, con diagnóstico explícito.

3. `nightly`
- DEBE considerarse experimental.
- NO DEBE prometer SLA.
- PUEDE incluir cambios en exploración con cobertura incremental.

## Política LTS empresarial

1. La designación LTS DEBE salir desde cortes `stable`.
2. La ventana LTS DEBE fijarse en `24 meses`.
3. El backport LTS DEBE limitarse a:
- seguridad;
- fallos críticos;
- riesgo de pérdida de datos.
4. El soporte enterprise DEBE declarar explícitamente alcance y SLA operativo.

## Gate oficial

- `release_phase10_support_policy_gate_tests`

## Pipeline de verificación

- `tools/ncc/nc/src/verification/support_policy.n`
- `tools/release/qa_phase10_p17_v2_0_0.sh`

## Criterio de cierre

F10-P17 queda cerrado cuando:

1. spec/compat/contract P17 están en `frozen`;
2. reporte `docs/status/F10_P17_support_policy_report.json` queda en `ok`;
3. `stable|beta|nightly` están tipados con su política de soporte canónica;
4. la ventana LTS empresarial queda definida sin ambigüedad;
5. el gate P17 queda integrado en QA global y release-train 2.x.
