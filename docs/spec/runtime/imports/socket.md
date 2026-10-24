# Import manual `use <net:socket>;`

`socket` expone handles crudos. El programador decide familia, tipo, protocolo,
buffers, flags, timeout, shutdown y cierre.

| Funcion | Firma |
|---|---|
| `socketCreate` | `(int, int, int) -> addr` |
| `socketConnect` | `(addr, str, int, int) -> int` |
| `socketBind` | `(addr, str, int) -> int` |
| `socketListen` | `(addr, int) -> int` |
| `socketAccept` | `(addr, addr, int) -> addr` |
| `socketSend` | `(addr, addr, int, int) -> int` |
| `socketRecv` | `(addr, addr, int, int) -> int` |
| `socketShutdown` | `(addr, int) -> int` |
| `socketSetTimeout` | `(addr, int, int) -> int` |
| `socketClose` | `(addr) -> int` |
| `socketLastError` | `() -> int` |

`socketConnect` usa resolucion nativa, modo no bloqueante, `WSAPoll`,
`SO_ERROR` y restaura el modo bloqueante. Los buffers de send/recv nunca se
crean ni crecen en el backend. Reusar un handle cerrado o una direccion
invalida sigue siendo responsabilidad raw del programa.
