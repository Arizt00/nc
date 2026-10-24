# nC Package Ecosystem Compatibility Spec v1.0.1

Estado: `frozen`  
Serie: `1.x`  
Fase: `phase7_package_ecosystem`

## Objetivo

Congelar compatibilidad de resolución de dependencias, lockfiles y registry para toda la serie `1.x`.

## Invariantes de compatibilidad

- `npkg.resolve.v1` y `npkg.lock.v1` no cambian de forma breaking en `1.x`.
- La política semver establecida en fase 7 no se relaja ni cambia sin RFC.
- La lógica de ordenamiento determinista de dependencias no cambia en `1.x`.
- `verify-lock` mantiene semántica estricta de drift detection.

## Política

- Cambios aditivos en metadata de lock/resolution: permitidos en `MINOR`.
- Cambios que alteren selección semver, orden determinista o schema base: solo `MAJOR`.
- Cualquier excepción requiere RFC aprobado y actualización de contratos.

## Gate de compatibilidad

- `release_phase7_package_ecosystem_gate_tests`

## Contrato machine-readable

- `docs/spec/release/v1_0_1_package_ecosystem_contract.json`
