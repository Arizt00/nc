# nC Governance + Release Cycle Compatibility Spec v1.0.1

Estado: `frozen`  
Serie: `1.x`  
Fase: `phase9_governance_release`

## Objetivo

Congelar compatibilidad del modelo institucional de releases para toda la serie `1.x`.

## Invariantes

- release train oficial mantiene `stable`, `beta` y `nightly`.
- cambios breaking siguen bloqueados en `1.x` salvo `MAJOR` + RFC.
- deprecaciones mantienen ventana mínima de 2 releases `stable` antes de remoción.
- LTS mantiene política de soporte/backport declarada.
- soporte empresarial mantiene tracks oficiales (`stable`/`lts`) y SLA publicados.
- la automatización CI/CD de promoción y de integridad de artefactos se mantiene activa.
- los warnings `NCC-DEPRECATION-*` de compilación se mantienen como superficie estable.
- el contrato SBOM/attestation (`ncc.sbom.v1` y `ncc.release.integrity.attestation.v1`) no cambia en breaking sin RFC.

## Política

- cambios aditivos en metadatos de gobernanza: permitidos en `MINOR`.
- cambios breaking de canal/cadencia/política de ruptura: solo `MAJOR` + RFC aprobado.
- cualquier variación en calendario, deprecación o SLA requiere actualización de contrato machine-readable y gates.

## Gate de compatibilidad

- `release_phase9_governance_release_gate_tests`

## Contrato machine-readable

- `docs/spec/release/v1_0_1_governance_release_contract.json`
