# Runtime Extreme E2E Closure

## Objetivo
Cerrar `[@runtime]` como contrato vivo de ejecución en nC, sin semántica inventada y con autoridad en `ncc`.

## Cadena end-to-end
`source -> lexer -> parser -> AST -> sema -> runtime contracts -> nIR -> lowering -> backend -> execution`.

## Garantías
- Import explícito de familias `nC.runtime.*`.
- Funciones runtime solo disponibles si la familia está importada.
- Catálogos canónicos de capabilities, policies y runtime errors.
- Diagnósticos estables (`E_RUNTIME_*`, `E_TYPE_*`, `E_CONDITION_*`).

## Fronteras de modo
- Permitido: `[@runtime]`, `[@tooling]`, rutas high-level autorizadas.
- Restringido/prohibido por policy: `[@]`, `[@machine]`, `[@kernel]`, `primitive[@]` para imports runtime no permitidos.

## CLI canónica
- `ncc runtime doctor`
- `ncc runtime families`
- `ncc runtime exports <family>`
- `ncc runtime check <file>`
- `ncc runtime imports <file>`
- `ncc runtime capabilities --json`
- `ncc runtime policies --json`
- `ncc runtime errors --json`
- `ncc runtime report --md-out <path>`

## Evidencia mínima de cierre
- `runtime_90_families_registered_ok`
- `runtime_all_capabilities_registered_ok`
- `runtime_all_policies_registered_ok`
- `runtime_all_errors_registered_ok`
- `runtime_import_nc_runtime_input_ok`
- `runtime_import_nc_runtime_output_ok`
