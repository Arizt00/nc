# Import `use <audit:bridge>`

## Contrato

`audit:bridge` representa fronteras materiales mediante una tabla raw que
pertenece al programa. El compilador no descubre bridges, no reserva la tabla,
no copia nombres y no impone ownership, capacidad, procedencia ni limites.

```n
use <audit:bridge>;
```

## Layout ABI

Cada entrada ocupa exactamente 32 bytes en todos los targets de 64 bits:

| Offset | Campo | Tipo material |
|---:|---|---|
| 0 | `key` | `i64` |
| 8 | `target` | `ptr` |
| 16 | `mode` | `i64` |
| 24 | `authority` | `i64` |

La clave numerica la elige el programa. La tabla puede residir en stack, heap,
memoria estatica o una region mapeada por el llamador.

## API raw

| Funcion | Firma | Efecto |
|---|---|---|
| `bridgeStride` | `() -> int` | devuelve `32` |
| `bridgeWrite` | `(table: addr, index: int, key: int, target: addr, mode: int, authority: int) -> bool` | escribe una entrada |
| `bridgeFind` | `(table: addr, count: int, key: int) -> int` | devuelve indice o `-1` |
| `bridgeActive` | `(table: addr, count: int, key: int) -> bool` | comprueba presencia |
| `bridgeRequire` | `(table: addr, count: int, key: int) -> int` | devuelve indice o ejecuta trap |
| `bridgeTarget` | `(table: addr, index: int) -> addr` | lee la direccion material |
| `bridgeMode` | `(table: addr, index: int) -> int` | lee el modo |
| `bridgeAuthority` | `(table: addr, index: int) -> int` | lee la autoridad |
| `bridgeErase` | `(table: addr, index: int) -> bool` | pone los 32 bytes logicos a cero |

## Semantica manual

No hay comprobacion de capacidad ni alineacion. Un indice fuera de rango, una
tabla nula o una direccion caducada se materializan como acceso raw y producen
comportamiento indefinido. `bridgeRequire` es la unica operacion con trap
deliberado: ocurre cuando la busqueda valida no encuentra la clave.

## Materializacion

La cadena es `.n -> IR -> LLVM/MCT -> Machine -> objeto -> ejecutable`. Las
nueve operaciones se implementan con loads, stores, comparaciones, ramas y
aritmetica de direcciones. No usan libc, strings administradas, serializacion,
allocators ni un proveedor C.

El ejemplo ejecutable es `examples/runtime/families/audit_bridge_raw.n`.
