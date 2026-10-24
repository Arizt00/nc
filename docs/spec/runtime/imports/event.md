# Import `use <task:event>`

## Contrato

`task:event` expone eventos del sistema como handles raw. No crea un bus de
eventos, listeners, futures ni scheduler. El programa decide seguridad, nombre
UTF-16, flags, arrays de handles, timeouts, waits alertables y cleanup.

```n
use <task:event>;
```

## API raw Win64

| Funcion | Firma | Efecto |
|---|---|---|
| `eventCreate` | `(attributes: addr, manualReset: bool, initial: bool, name: addr) -> addr` | crea un evento |
| `eventCreateEx` | `(attributes: addr, name: addr, flags: int, access: int) -> addr` | crea un evento con flags/acceso explicitos |
| `eventOpen` | `(access: int, inherit: bool, name: addr) -> addr` | abre un evento nombrado |
| `eventSet` | `(event: addr) -> bool` | pone el evento en estado senalado |
| `eventReset` | `(event: addr) -> bool` | limpia el estado senalado |
| `eventPulse` | `(event: addr) -> bool` | ejecuta la operacion heredada `PulseEvent` |
| `eventWait` | `(event: addr, timeout: int) -> int` | espera un handle |
| `eventWaitEx` | `(event: addr, timeout: int, alertable: bool) -> int` | espera alertable |
| `eventWaitMany` | `(count: int, handles: addr, all: bool, timeout: int) -> int` | espera un array raw de handles |
| `eventWaitManyEx` | `(count: int, handles: addr, all: bool, timeout: int, alertable: bool) -> int` | espera multiple alertable |
| `eventSignalAndWait` | `(signal: addr, wait: addr, timeout: int, alertable: bool) -> int` | senala y espera atomicamente |
| `eventClose` | `(event: addr) -> bool` | cierra el handle |
| `eventLastError` | `() -> int` | lee el ultimo error Win32 |

Los codigos de wait se devuelven sin traducir: `0` es `WAIT_OBJECT_0`, `258` es
`WAIT_TIMEOUT` y `0xFFFFFFFF` es `WAIT_FAILED`. Handles invalidos, arrays con
capacidad falsa o uso posterior a `eventClose` conservan la semantica raw del
sistema operativo.

## Materializacion

La cadena es `.n -> IR -> LLVM/MCT -> Machine -> COFF -> PE64`. Los wrappers
`__ncc_event_*` llaman directamente a `KERNEL32.dll`. No se genera C, no se
enlaza CRT y no existe asignacion o scheduling oculto.

El ejemplo ejecutable es `examples/runtime/families/task_event_raw.n`.

## Targets

El cierre actual es Win64 x86_64/AArch64. Otro target recibe
`E_MCT_LLVM_TASK_EVENT_TARGET_001`; no se finge un backend POSIX.
