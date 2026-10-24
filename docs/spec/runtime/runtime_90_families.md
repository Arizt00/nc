# Runtime: 90 familias base + 3 extensiones nativas

## Regla
El catálogo conserva **90 familias base** y añade tres extensiones nativas tipadas: `nC.runtime.math`, `nC.runtime.terminal` y `nC.runtime.json`. El total operativo es **93** y se valida por test y por `runtime doctor`.

## Verificación
- `ncc runtime families`
- `ncc runtime doctor`

## Contrato
- Cada familia define exports/signatures.
- Cada familia define required capabilities/policies cuando aplica.
- Ninguna familia existe solo como nombre: debe tener surface verificable.

## Ejemplos nucleares
- `nC.runtime.input`
- `nC.runtime.output`
- `nC.runtime.parse`
- `nC.runtime.render`
- `nC.runtime.memory`
- `nC.runtime.capability`
- `nC.runtime.policy`
- `nC.runtime.math`
- `nC.runtime.terminal`
- `nC.runtime.json`
