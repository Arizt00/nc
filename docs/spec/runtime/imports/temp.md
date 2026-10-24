# Import manual `process:temp`

## Contrato

```n
use <process:temp>;
```

Expone rutas, archivos y directorios temporales como buffers UTF-16 y handles
raw. El programa decide capacidades, nombres, permisos, flags y limpieza. No se
crean `str`, objetos `File`, destructores ni rutas administradas.

## Superficie

| Operacion | Firma |
|---|---|
| `tempWideAlloc` | `(str) -> addr` |
| `tempWideBuffer` | `(int) -> addr` |
| `tempWideFree` | `(addr) -> bool` |
| `tempDirectory` | `(addr, int) -> int` |
| `tempFileName` | `(addr, addr, int, addr) -> int` |
| `tempOpen` | `(addr, int, int, addr, int, int, addr) -> addr` |
| `tempDeleteFile` | `(addr) -> bool` |
| `tempCreateDirectory` | `(addr, addr) -> bool` |
| `tempRemoveDirectory` | `(addr) -> bool` |
| `tempMove` | `(addr, addr, int) -> bool` |
| `tempAttributes` | `(addr) -> int` |
| `tempSetAttributes` | `(addr, int) -> bool` |
| `tempFinalPath` | `(addr, addr, int, int) -> int` |
| `tempClose` | `(addr) -> bool` |
| `tempLastError` | `() -> int` |

## Materializacion Win64

El lowering llama directamente a `GetTempPathW`, `GetTempFileNameW`,
`CreateFileW`, `DeleteFileW`, `CreateDirectoryW`, `RemoveDirectoryW`,
`MoveFileExW`, atributos, ruta final y `CloseHandle` en `KERNEL32.dll`. No usa
`tmpfile`, `tempnam`, CRT ni un proveedor C.

## Semantica manual

- Las capacidades se expresan en caracteres UTF-16, no en bytes.
- `tempFileName(..., unique: 0, ...)` conserva la creacion atomica de Win32.
- Un handle devuelto por `tempOpen` debe cerrarse con `tempClose`.
- Los buffers de `tempWideAlloc/Buffer` deben liberarse con `tempWideFree`.
- Ningun fallo activa cleanup, reintento, renombrado o borrado automatico.
- Handles, direcciones, flags y tamanos invalidos se envian al sistema tal cual.

## Evidencia

El ejemplo [process_temp_raw.n](../../../../examples/runtime/families/process_temp_raw.n)
reserva buffers, obtiene el directorio temporal, crea un archivo unico, consulta
atributos, abre el handle, resuelve la ruta final, cierra, borra y libera cada
buffer de forma explicita.
