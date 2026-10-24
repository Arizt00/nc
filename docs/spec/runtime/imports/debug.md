# Import `use <audit:debug>`

## Contrato

`audit:debug` ofrece registros, dumps, checksum y traps sobre memoria raw del
programa. No imprime, no reserva memoria, no captura automaticamente registros
o stack y no crea valores `any` ni mensajes administrados.

```n
use <audit:debug>;
```

## Layout ABI

Cada registro ocupa 32 bytes: `tag:i64` en offset 0, `location:ptr` en 8,
`value:i64` en 16 y `flags:i64` en 24.

## API raw

| Funcion | Firma |
|---|---|
| `debugStride` | `() -> int` |
| `debugRecord` | `(buffer: addr, index: int, tag: int, location: addr, value: int, flags: int) -> bool` |
| `debugTag` | `(buffer: addr, index: int) -> int` |
| `debugLocation` | `(buffer: addr, index: int) -> addr` |
| `debugValue` | `(buffer: addr, index: int) -> int` |
| `debugFlags` | `(buffer: addr, index: int) -> int` |
| `debugDump` | `(destination: addr, source: addr, bytes: int) -> int` |
| `debugChecksum` | `(source: addr, bytes: int) -> int` |
| `debugBreak` | `() -> bool` |
| `debugAssert` | `(condition: bool, context: addr, bytes: int) -> bool` |

`debugDump` copia byte a byte y `debugChecksum` calcula FNV-1a de 64 bits. El
contexto de `debugAssert` permanece propiedad del programa y no es leido por el
backend. `debugBreak` y el fallo de `debugAssert` ejecutan trap.

## Semantica manual

No hay capacidad, alineacion, lifetime ni bounds checks. Direcciones o indices
invalidos se materializan y tienen comportamiento indefinido. El programa
decide donde almacenar, presentar o persistir la informacion.

## Materializacion

La familia baja a loads, stores, ramas y aritmetica LLVM/Machine. No usa
`malloc`, `memcpy`, logging, serializacion, libc ni proveedor C. El ejemplo real
es `examples/runtime/families/audit_debug_raw.n`.
