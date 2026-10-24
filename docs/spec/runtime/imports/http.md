# Import manual `use <net:http>;`

## Contrato

`http` expone una respuesta opaca asignada por el backend. El programador es
responsable de conservar y liberar su direccion; no hay ownership, cierre,
crecimiento ni copia implicitos.

```n
use <net:http>;

response >> addr <- httpGet("https://example.org/");
status >> int <- httpStatus(response);
data >> addr <- httpBodyData(response);
bytes >> int <- httpBodyBytes(response);
error >> int <- httpError(response);
httpRelease(response);
```

## Superficie

| Funcion | Firma | Contrato |
|---|---|---|
| `httpGet` | `(str) -> addr` | GET con timeout y limite predeterminados |
| `httpPost` | `(str, str) -> addr` | POST UTF-8 con timeout y limite predeterminados |
| `httpRequestRaw` | `(str, str, str, addr, int, int, int) -> addr` | metodo, URL, headers, cuerpo, bytes, timeout y limite explicitos |
| `httpStatus` | `(addr) -> int` | codigo HTTP; cero si no hubo respuesta valida |
| `httpBody` | `(addr) -> str` | vista UTF-8 prestada, terminada en cero |
| `httpBodyData` | `(addr) -> addr` | direccion prestada de los bytes recibidos |
| `httpBodyBytes` | `(addr) -> int` | longitud material, sin contar el terminador |
| `httpError` | `(addr) -> int` | codigo de error del sistema |
| `httpRelease` | `(addr) -> bool` | libera cuerpo y descriptor |

La direccion de `httpBody` y `httpBodyData` deja de ser valida al llamar a
`httpRelease`. El modo raw no impide reutilizarla despues: el resultado es
comportamiento indefinido, como cualquier use-after-free manual.

## Backend

En `x86_64-pc-windows-msvc` el compilador genera LLVM y Machine que llaman
directamente a WinHTTP. TLS conserva la verificacion del sistema y los
redirects de WinHTTP. El proveedor usa el allocator raw de nC y no genera,
compila ni enlaza fuente C.

Los targets sin renderer HTTP nativo se rechazan con
`E_MCT_LLVM_HTTP_TARGET_001`; no existe fallback silencioso a curl, libc ni un
interprete.

## Errores

Una reserva o validacion de argumentos fallida devuelve `0`. Si existe un
descriptor, `httpError` conserva el error Win32 y `httpStatus` queda en cero.
El cuerpo parcial se mantiene hasta `httpRelease`.

## Evidencia

- ejemplo: `examples/runtime/families/http_basic.n`;
- sema: `crates/ncc-sema/tests/compact_net_http.rs`;
- IR/LLVM/Machine: prueba `stoud_net_http_lowers_to_direct_winhttp_and_nc_machine_calls`;
- artefactos: `target/evidence/imports/stoud-net-http-20260802-winhttp-crtfree-final`.
