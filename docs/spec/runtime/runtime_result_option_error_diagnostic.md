# Runtime Result/Option/Error/Diagnostic

## Result/Option
Soporte de rutas `ok/err` y `some/none` con control de unwrap inseguro.

## Error/Diagnostic
`dev(error @runtime)` y catálogos de diagnóstico tipados.

## Reglas
- `unwrap` inseguro en ruta err/none debe diagnosticar.
- Errores sin código estable no se aceptan.
