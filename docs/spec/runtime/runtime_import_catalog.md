# Runtime Import Catalog

## Import canónico
Se usa `use nC.runtime.<family>;`.

## Reglas
- No hay import implícito.
- No hay wildcard inseguro por defecto.
- El resolver exige path canónico.
- El uso de función runtime sin import válido falla.

## Diagnósticos relacionados
- `E_RUNTIME_IMPORT_001`
- `E_RUNTIME_IMPORT_002`
- `E_RUNTIME_USE_001`

## Comandos
- `ncc runtime imports <file>`
- `ncc runtime check <file>`
