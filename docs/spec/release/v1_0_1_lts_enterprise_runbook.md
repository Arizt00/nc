# nC LTS + Enterprise Runbook v1.0.1

Estado: `frozen`  
Serie: `1.x`

## SLA operativo

- Security response SLA: <= 24h
- Critical hotfix target: <= 72h
- Ventana LTS: 24 meses
- Remoción por deprecación: mínimo 2 releases `stable` desde anuncio

## Flujo operativo

1. Intake y clasificación: `security`, `critical-bug`, `data-loss`, `standard`.
2. Confirmar track (`stable` o `lts`) y alcance de impacto.
3. Abrir incidente, asignar owner y activar reloj de SLA.
4. Preparar fix y backport según política LTS.
5. Ejecutar gates de release y verificación de integridad de artefactos.
6. Publicar release notes/changelog con impacto y ventana de compatibilidad.

## Validación ejecutable

- `tools/release/runbook_lts_enterprise_sla_v1_0_1.sh`

Este script valida automáticamente los umbrales y contratos mínimos en
`docs/spec/release/v1_0_1_governance_release_contract.json`.
