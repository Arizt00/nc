# Import manual `process:signal`

## Contrato

```n
use <process:signal>;
```

Expone sincronizacion, eventos de consola, APC y manejo de excepciones de
Windows como operaciones raw. El programa aporta handles, callbacks, buffers,
codigos, tiempos de espera y memoria; el compilador no instala handlers, no
crea hilos y no gestiona la vida de ningun recurso.

## Superficie

| Operacion | Firma |
|---|---|
| `signalWideAlloc` | `(str) -> addr` |
| `signalWideFree` | `(addr) -> bool` |
| `signalCtrlHandler` | `(addr, bool) -> bool` |
| `signalCtrlEvent` | `(int, int) -> bool` |
| `signalCreateEvent` | `(addr, bool, bool, addr) -> addr` |
| `signalOpenEvent` | `(int, bool, addr) -> addr` |
| `signalSetEvent` | `(addr) -> bool` |
| `signalResetEvent` | `(addr) -> bool` |
| `signalPulseEvent` | `(addr) -> bool` |
| `signalWaitOne` | `(addr, int) -> int` |
| `signalWaitOneEx` | `(addr, int, bool) -> int` |
| `signalWaitMany` | `(int, addr, bool, int) -> int` |
| `signalWaitManyEx` | `(int, addr, bool, int, bool) -> int` |
| `signalQueueApc` | `(addr, addr, int) -> int` |
| `signalAddVectored` | `(int, addr) -> addr` |
| `signalRemoveVectored` | `(addr) -> bool` |
| `signalSetUnhandledFilter` | `(addr) -> addr` |
| `signalRaiseException` | `(int, int, int, addr) -> bool` |
| `signalClose` | `(addr) -> bool` |
| `signalLastError` | `() -> int` |

## Materializacion Win64

El lowering llama directamente a `KERNEL32.dll`: `CreateEventW`,
`OpenEventW`, `WaitForSingleObject`, `WaitForMultipleObjectsEx`, `QueueUserAPC`,
`SetConsoleCtrlHandler`, `GenerateConsoleCtrlEvent`, los handlers de excepcion
vectorizados y `RaiseException`. No usa `signal`, `raise`, CRT ni un proveedor C.

`signalWideAlloc` convierte UTF-8 a UTF-16 en memoria raw alineada. Esa memoria
pertenece al programa y se libera exclusivamente con `signalWideFree`.

## Semantica manual

- `addr` puede ser nulo, invalido, colgante o apuntar a memoria incompatible.
- `signalWaitMany*` recibe un array raw de handles y confia en `count`.
- Los callbacks deben respetar la ABI de Windows y permanecer ejecutables.
- `signalRaiseException` puede transferir el control o terminar el proceso.
- `signalPulseEvent` conserva exactamente la semantica fragil de Win32.
- Los codigos de espera se devuelven sin reinterpretar, por ejemplo `0`, `258`
  o `0xFFFFFFFF`.

## Evidencia

El ejemplo [process_signal_raw.n](../../../../examples/runtime/families/process_signal_raw.n)
ejecuta crear, timeout, set, wait, reset, timeout y close. Las rutas de control,
APC y excepciones se certifican por tipado, IR, llamadas Machine e imports PE;
no se disparan durante el smoke porque alterarian el proceso de prueba.
