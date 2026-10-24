# Runtime Mode Restrictions

## Ley
`[@runtime]` no se infiltra automáticamente en otros modos.

## Restricciones clave
- `primitive[@]`: runtime import prohibido.
- `[@machine]`/`[@kernel]`: input runtime restringido por policy.
- `[@]` root-basal: solo rutas permitidas por contrato.

## Diagnósticos
- `E_RUNTIME_MODE_001`
- `E_MODE_BOUNDARY_001`
- `E_RUNTIME_PATH_FORBIDDEN_001`
