# `auth.system:permission`

## Import

```n
use <auth.system:permission>;
```

## Contrato

Esta familia representa decisiones de autoridad en memoria raw aportada por el
programa. No solicita permisos al sistema operativo, no crea objetos, no posee
la tabla y no aplica checks de rango o vida util.

Cada registro ocupa 64 bytes:

| Offset | Campo | Tipo material |
|---:|---|---|
| 0 | subject | i64 |
| 8 | resource | i64 |
| 16 | rights | i64 |
| 24 | denied | i64 |
| 32 | flags | i64 |
| 40 | owner | i64 |
| 48 | generation | i64 |
| 56 | state | i64 |

## Operaciones

`authPermissionStride`, `authPermissionWrite`, `authPermissionSubject`,
`authPermissionResource`, `authPermissionRights`, `authPermissionDenied`,
`authPermissionFlags`, `authPermissionOwner`, `authPermissionGeneration`,
`authPermissionState`, `authPermissionAllows`, `authPermissionCopy` y
`authPermissionClear`.

`authPermissionAllows(table, index, required)` devuelve verdadero cuando el
registro esta activo, contiene todos los bits requeridos y no deniega ninguno.
El acceso a una direccion, indice o vida util invalidos conserva la semantica
raw elegida por el programador.

## Materializacion

Las 13 operaciones tienen cuerpos LLVM internos y destinos Machine
`__ncc_auth_permission_*`. No usan C, CRT, heap, strings administradas ni un
proveedor externo.

Evidencia ejecutable:
`examples/runtime/families/auth_system_permission_manual_raw.n`.
