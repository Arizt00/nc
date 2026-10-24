# Import `use <audit:assert>`

## Contrato

`audit:assert` expone comprobaciones explicitas que terminan la ejecucion con
una instruccion trap cuando la condicion no se cumple. No imprime, no formatea,
no reserva memoria, no captura stack y no instala un manejador de errores.

```n
use <audit:assert>;
```

## API raw

| Funcion | Firma | Efecto |
|---|---|---|
| `assert` | `(condition: bool) -> bool` | devuelve `true` o ejecuta trap |
| `assertMsg` | `(condition: bool, message: addr, bytes: int) -> bool` | conserva contexto raw del llamador y ejecuta trap si falla |
| `assertEq` | `(left: int, right: int) -> bool` | compara enteros con signo |
| `assertEqAddr` | `(left: addr, right: addr) -> bool` | compara direcciones |
| `assertNotNull` | `(value: addr) -> bool` | exige direccion no nula |
| `assertRange` | `(value: int, minimum: int, maximum: int) -> bool` | exige rango inclusivo |
| `assertFail` | `() -> bool` | ejecuta trap incondicional |
| `assertUnreachable` | `() -> bool` | marca y ejecuta una ruta inalcanzable |

`assertMsg` recibe un puntero y una longitud; no acepta `str` administrada ni
crea una copia. El backend no lee ni presenta el mensaje. Si el programa quiere
logging, debe hacerlo explicitamente antes de llamar a la asercion.

## Materializacion

La cadena es `.n -> IR -> LLVM/MCT -> Machine -> objeto -> ejecutable`. Los ocho
simbolos `__ncc_assert_*` contienen control flow tipado y `llvm.trap`; no llaman
a `assert` de libc, `abort`, `printf` ni un runtime C.

El ejemplo ejecutable es `examples/runtime/families/audit_assert_raw.n`.

## Targets

El contrato usa una primitiva LLVM portable. La instruccion final de trap la
selecciona el backend para cada target; una condicion falsa termina de forma
anormal y su codigo concreto depende del sistema operativo y arquitectura.
