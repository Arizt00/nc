# Import manual `process:env`

## Superficie

```n
use <process:env>;
```

La familia opera sobre el entorno del proceso sin `getenv`, `putenv`, CRT ni
proveedor C. Enumerar es una operacion material: `envOpen` crea un descriptor
propio y `envRelease` destruye sus cadenas UTF-8, vector y cabecera.

## ABI raw

| Funcion | Firma | Contrato |
|---|---|---|
| `envOpen` | `() -> addr` | Enumera y convierte el bloque nativo. |
| `envCount` | `(addr) -> int` | Numero de entradas. |
| `envAt` | `(addr, int) -> str` | Vista prestada sin bounds check. |
| `envData` | `(addr) -> addr` | Vector raw de punteros UTF-8. |
| `envBytes` | `(addr) -> int` | Bytes de entradas, incluidos terminadores. |
| `envError` | `(addr) -> int` | Error de apertura/conversion. |
| `envRelease` | `(addr) -> bool` | Libera todo el descriptor. |
| `envGet` | `(str, addr, int) -> int` | Mide o escribe en un buffer del llamador. |
| `envHas` | `(str) -> bool` | Incluye variables con valor vacio. |
| `envSet` | `(str, str) -> bool` | Crea o reemplaza una variable. |
| `envRemove` | `(str) -> bool` | Elimina una variable. |
| `envExpand` | `(str, addr, int) -> int` | Expande `%NAME%` en un buffer raw. |
| `envLastError` | `() -> int` | Devuelve el error nativo del hilo. |

`envGet` y `envExpand` devuelven los bytes escritos sin el cero final. Con
destino nulo o capacidad insuficiente devuelven los bytes requeridos incluido
el cero. Un fallo devuelve el codigo nativo negado. El programador decide la
reserva, capacidad, aliasing, validez y vida del buffer.

## Materializacion Win64

El backend llama directamente a `GetEnvironmentStringsW`,
`FreeEnvironmentStringsW`, `GetEnvironmentVariableW`,
`SetEnvironmentVariableW`, `ExpandEnvironmentStringsW`,
`MultiByteToWideChar` y `WideCharToMultiByte`. La reserva interna solicitada
por `envOpen` usa la autoridad raw `VirtualAlloc`/`VirtualFree`.

La evidencia ejecutable esta en
`target/evidence/imports/stoud-process-env-20260802-win64-crtfree-final`.
