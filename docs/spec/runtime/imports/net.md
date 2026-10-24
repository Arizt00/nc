# Import manual `use <net:net>;`

`net` controla la inicializacion del subsistema de red y escribe resoluciones o
interfaces en memoria proporcionada por el programa. No asigna colecciones ni
libera recursos automaticamente.

| Funcion | Firma |
|---|---|
| `netStartup` | `() -> int` |
| `netShutdown` | `() -> int` |
| `netLastError` | `() -> int` |
| `netAvailable` | `() -> bool` |
| `netResolve` | `(str, int, addr, int) -> int` |
| `netAddressOf` | `(str, int, addr, int) -> int` |
| `netInterfaces` | `(addr, int) -> int` |

`netResolve` y `netAddressOf` escriben un `sockaddr` nativo y devuelven sus
bytes. Un valor negativo es un error; si el buffer es pequeno, su magnitud es la
capacidad requerida.

`netInterfaces` escribe registros autocontenidos de 144 bytes. Cada registro
contiene `flags: u32` en el offset 0, `family: u32` en el 4,
`addressLength: u64` en el 8 y hasta 128 bytes de `sockaddr` en el 16. Una
consulta con buffer nulo o insuficiente devuelve el tamano requerido negado.
Ningun puntero de `getifaddrs` sobrevive a la llamada.

El lowering es material: Win64 usa Winsock/IP Helper y Linux, Android y Darwin
de 64 bits usan sockets POSIX, `getaddrinfo`, `poll`, `fcntl` y `getifaddrs`.
El compilador inserta el proveedor cuando encuentra la familia `net/socket` o
la compatibilidad `network::*`; no compila un proveedor C para estas funciones.
