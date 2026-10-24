# Import manual `process:exit`

## Superficie

```n
use <process:exit>;
```

No existe un codigo de salida administrado por el runtime. El programador usa
handles de proceso, permisos de apertura y codigos nativos de forma explicita.

## ABI raw

| Funcion | Firma | Contrato |
|---|---|---|
| `processCurrent` | `() -> addr` | Pseudohandle del proceso actual. |
| `processOpen` | `(int, bool, int) -> addr` | Access mask, herencia y PID. |
| `processId` | `(addr) -> int` | PID asociado al handle. |
| `processExitCode` | `(addr) -> int` | Codigo o error nativo negado. |
| `processTerminate` | `(addr, int) -> bool` | Fuerza la terminacion del handle. |
| `processClose` | `(addr) -> bool` | Cierra un handle poseido. |
| `exitNow` | `(int) -> bool` | Termina inmediatamente el proceso actual. |
| `failFast` | `(addr, addr, int) -> bool` | Pasa record, contexto y flags raw. |
| `processLastError` | `() -> int` | Error nativo del hilo. |

`exitNow` y `failFast` tienen tipo `bool` para cruzar la expresion ABI, pero
una ejecucion correcta no retorna. No ejecutan `defer`, destructores, flush,
atexit ni limpieza implicita.

## Materializacion Win64

El backend usa directamente `GetCurrentProcess`, `OpenProcess`, `GetProcessId`,
`GetExitCodeProcess`, `TerminateProcess`, `CloseHandle`, `ExitProcess` y
`RaiseFailFastException` de `KERNEL32.dll`. No llama `exit`, `_exit`, `abort` ni
funciones del CRT.

La evidencia esta en
`target/evidence/imports/stoud-process-exit-20260802-win64-crtfree-final`.
