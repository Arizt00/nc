# Import manual `process:args`

## Superficie

```n
use <process:args>;
```

El import habilita acceso material a los argumentos del proceso. No crea una
lista administrada, no conserva ownership oculto y no programa limpieza.

| Funcion | Contrato | Memoria |
|---|---|---|
| `argsOpen` | `() -> addr` | Reserva descriptor, vector y cadenas UTF-8. |
| `argsCount` | `(addr) -> int` | Lee el numero de entradas. |
| `argsAt` | `(addr, int) -> str` | Devuelve un prestamo crudo sin bounds check. |
| `argsData` | `(addr) -> addr` | Expone el vector material de punteros. |
| `argsBytes` | `(addr) -> int` | Suma bytes UTF-8, terminadores incluidos. |
| `argsError` | `(addr) -> int` | Lee el error nativo retenido. |
| `argsRelease` | `(addr) -> bool` | Libera cadenas, vector y descriptor. |

## Semantica raw

`argsOpen` es la unica operacion que reserva memoria. El programador debe
llamar `argsRelease` exactamente cuando decida terminar la vida del descriptor.
`argsAt` no valida el indice: salir de rango, usar el resultado tras liberar o
liberar dos veces conserva comportamiento indefinido deliberado.

Layout Win64 del descriptor:

| Offset | Campo | Tipo |
|---:|---|---|
| 0 | count | `u64` |
| 8 | data | `addr` |
| 16 | bytes | `u64` |
| 24 | error | `u64` |

## Backend

La ruta Win64 llama directamente a `GetCommandLineW`, `CommandLineToArgvW`,
`WideCharToMultiByte` y `LocalFree`. La memoria se obtiene mediante el allocator
raw LLVM/Machine respaldado por `VirtualAlloc`/`VirtualFree`.

No se genera fuente C, no se compila un proveedor C y no se importa CRT. El PE
final solo requiere `KERNEL32.dll` y `SHELL32.dll` para esta familia.

## Ejemplo

```n
use <process:args>;

handle >> addr <- argsOpen();
count >> int <- argsCount(handle);
first >> str <- argsAt(handle, 1);
argsRelease(handle);
```

El ejemplo ejecutable completo es
`examples/runtime/families/process_args_raw.n`.

## Evidencia

- parser y sema positivos;
- indice no entero rechazado;
- enlaces IR canonicos;
- definiciones LLVM directas;
- llamadas Machine conservadas;
- objeto COFF y PE enlazados;
- ejecucion real con argumentos UTF-8 y espacios;
- tabla de imports sin CRT ni proveedor C.

La ejecucion retenida de esta iteracion certifica Windows x86_64. La misma ABI
LLVM admite Windows AArch64; su ejecucion requiere un runner Arm64 real.
