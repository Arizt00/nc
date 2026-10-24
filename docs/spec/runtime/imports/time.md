# Tiempo del sistema

La familia `nC.runtime.time` pertenece al prelude hosted y no requiere `use`.

## Operaciones

| Funcion | Firma | Unidad/propiedad |
|---|---|---|
| `now`, `timestamp` | `() -> int` | Segundos Unix UTC |
| `nowMs` | `() -> int` | Milisegundos Unix UTC |
| `monotonicMs` | `() -> int` | Reloj monotonicamente creciente |
| `monotonicNs` | `() -> int` | Nanosegundos monotónicos |
| `sleep` | `(int) -> void` | Suspensión explícita en milisegundos |
| `sleepUntilMs` | `(int) -> bool` | Espera hasta un instante monotónico |
| `formatTimeInto` | `(int, p<byte>, u64) -> int` | UTC en memoria del llamador |
| `dateInto` | `(p<byte>, u64) -> int` | UTC actual en memoria del llamador |
| `formatTime`, `date` | formas compatibles que devuelven `str` |
| `timeRelease` | `(str) -> bool` | libera texto de compatibilidad |

`formatTimeInto` y `dateInto` requieren 24 bytes y escriben exactamente
`YYYY-MM-DD HH:MM:SS UTC` más el cero final. Devuelven 23, 17 para
`date-out-of-range`, o `-1` si el buffer material no sirve.

No se asigna, redimensiona ni libera memoria en las formas `Into`. La
procedencia y vida del puntero pertenecen al programa raw.

## Backend

- Windows: `GetSystemTimeAsFileTime`, `QueryPerformanceCounter`,
  `QueryPerformanceFrequency` y `Sleep`.
- Linux/Android: `clock_gettime`, `clock_getres` y `usleep`, con layouts de 32
  y 64 bits diferenciados.
- Darwin: `gettimeofday`, `mach_absolute_time`, `mach_timebase_info` y
  `usleep`.
- WASI: `clock_time_get`, `clock_res_get` y `poll_oneoff`.

El formateo UTC está escrito en LLVM/MCT y no depende de libc ni del runtime C.
