# Imports manuales `auth.resource`

## Superficie

```nC
use <auth.resource:handle, resource, session>;
```

Los tres imports son distintos. No reexportan las APIs hosted antiguas de
sesiones o recursos gestionados.

## Handle

Cada descriptor ocupa 40 bytes: value, kind, rights, flags y generation.
`handleWrite`, lectores de campo, `handleAllows`, `handleCopy` y `handleClear`
operan sobre una tabla aportada por el programa.

## Resource

Cada descriptor ocupa 64 bytes: id, handle, base, bytes, kind, rights, flags y
owner. `resourceContains` comprueba un intervalo solo cuando se invoca;
`resourceAllows` compara máscaras explícitas. No se adquiere ni libera el
recurso del sistema.

## Session

Cada descriptor ocupa 56 bytes: id, owner, token, tabla de recursos, count,
rights y state. `sessionResourceAt` calcula `resources + index * stride` sin
bounds ni `inbounds`. El llamador aporta stride, count, memoria y sincronización.

## Semántica raw

No hay ownership, lifetime, allocation, locks, política automática, validación
de handles ni cierre implícito. Direcciones o índices inválidos producen el
comportamiento indefinido propio del modo raw. Las 37 operaciones se
materializan directamente en LLVM/Machine y no tienen proveedor C.

Ejemplo ejecutable: `examples/runtime/families/auth_resource_manual_raw.n`.

