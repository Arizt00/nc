# Compiler Type Resolver

## Cobertura
Tipos base, genéricos, uniones, ptr/region, arrays/tensores, Result/Option, void/null.

Incluye resolución y validación de `type:` contract sections:

- campos con visibilidad contractual (`public/private/internal/strictly`),
- cláusulas internas (`layout/bounds/invariant/constructure/destructure/copy/clone/move/ffi`),
- cláusulas extendidas (`ownership/nullability/methods/associated/validate/drop/default/compare/hash/debug/stable`),
- chequeos semánticos de contrato con diagnósticos `E_TYPE_*`.

CLI relacionado:

- `ncc compiler types <file> --json`
- `ncc compiler type-contracts <file> --json`

## Regla
Toda operación debe resolver tipo de entrada y salida o fallar con código estable.
