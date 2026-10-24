# nC DX + Observability Compatibility Spec v1.0.1

Estado: `frozen`  
Serie: `1.x`  
Fase: `phase8_dx_observability`

## Objetivo

Congelar compatibilidad de DX y observabilidad para toda la serie `1.x`.

## Invariantes

- códigos `NCC-CLI-*` y `NCC-DX-*` no cambian de forma breaking sin RFC.
- `ncc.dx.profile.v1` mantiene contrato de campos críticos.
- `nide.diagnostics.v1` mantiene `error_entries` con `code/phase/message/hint`.
- sugerencias automáticas se mantienen activas para errores comunes de comando/acción.

## Política

- cambios aditivos de metadata en perfiles/diagnósticos: permitidos en `MINOR`.
- cambios breaking de códigos, schemas o etapas: solo `MAJOR`.
- excepciones: RFC aprobado + actualización de contratos.

## Gate de compatibilidad

- `release_phase8_dx_observability_gate_tests`

## Contrato machine-readable

- `docs/spec/release/v1_0_1_dx_observability_contract.json`
