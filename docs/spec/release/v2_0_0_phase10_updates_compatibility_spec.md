# nC Phase10 Updates Compatibility Spec v2.0.0

Estado: `frozen`  
Serie: `2.x`

## Compatibilidad activa

- `ncup update` DEBE mantener semántica de actualización de toolchain activa efectiva.
- La transacción con rollback automático DEBE permanecer obligatoria en serie `2.x`.
- `manifests/update-history.v1.json` DEBE mantenerse como artefacto estable de observabilidad.
- La política `validate-before-commit` NO DEBE degradarse.

## Reglas

1. Cambios breaking en transacción/rollback/historial requieren RFC.
2. Nuevos campos en historial PUEDEN añadirse de forma aditiva.
3. El comando `ncup update` PUEDE extender opciones, pero NO DEBE romper defaults canónicos.
