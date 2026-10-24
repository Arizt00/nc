# nC Package Ecosystem Spec v1.0.1

Estado: `frozen`  
Serie: `1.x`  
Fase: `phase7_package_ecosystem`

## Objetivo

Cerrar el ecosistema de paquetes y builds de nC con comportamiento de producción:

- resolver dependencias estable;
- lockfile determinista;
- contrato semver canónico;
- registry JSON verificable;
- reproducibilidad de resolución en proyectos externos sin hacks.

## Superficie `npkg` canónica

- `npkg manifest <file.n> [--out <path>]`
- `npkg check-imports <file.n>`
- `npkg list-imports <file.n>`
- `npkg resolve <file.n> [--registry <path>] [--out <path>]`
- `npkg lock <file.n> [--registry <path>] [--lock <path>]`
- `npkg verify-lock <file.n> [--registry <path>] --lock <path>`

## Contratos obligatorios

- registry schema: `npkg.registry.v1`
- resolution schema: `npkg.resolve.v1`
- lock schema: `npkg.lock.v1`
- semver policy: `major.minor.patch` estricto + rangos `^`, `~`, wildcard y comparadores
- selección de versión: máxima versión válida por semver requirement
- determinismo: orden estable de dependencias en lock/resolution

## Reglas de estabilidad

- mismo `file.n` + mismo registry + misma versión de toolchain => mismo lock byte a byte;
- `verify-lock` debe fallar si existe drift de resolución;
- no breaking changes de schemas ni del contrato semver sin RFC.

## DoD de Fase 7

Fase 7 se considera cerrada solo si:

1. spec + compat + contract están en `frozen`;
2. gate `release_phase7_package_ecosystem_gate_tests` está en verde;
3. `npkg lock` es determinista y `npkg verify-lock` detecta drift;
4. proyectos externos (cwd limpio, fuera del repo) resuelven/lockean/compilan reproduciblemente;
5. contrato legacy referencia gate y contrato de fase 7.

## Gate obligatorio

- `release_phase7_package_ecosystem_gate_tests`
