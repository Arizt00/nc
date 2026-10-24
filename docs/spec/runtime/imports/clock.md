# Reloj monotónico

La familia `nC.runtime.clock` pertenece al prelude hosted. El import
`use <time:clock>;` sigue aceptado, pero no es necesario.

```n
start >> u64 <- monotonic();
delta >> u64 <- elapsed(start);
```

| Funcion | Firma | Semantica |
|---|---|---|
| `monotonic` | `() -> u64` | instante monotónico en nanosegundos |
| `elapsed` | `(u64) -> u64` | resta saturada contra el instante actual |
| `measure` | `(fn() -> bool) -> u64` | invoca una vez el puntero de funcion |
| `clockResolution` | `() -> u64` | resolución material en nanosegundos |

No hay hilo, scheduler, asignación ni captura implícita. `measure` usa el ABI de
puntero de función del target y ejecuta el callback sin intermediario C.
