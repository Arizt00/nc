# Contrato raw de memoria runtime

Estado: vigente para `buffer`, `slice`, `span`, `alloc`, `bytes`, `layout` y
`endian`.

## Regla material

`buffer`, `slice` y `span` son descriptores binarios propiedad del programa. No
son handles, no tienen ownership, no devuelven `Result` y no realizan bounds
checks. El programa reserva, inicializa, comparte y libera tanto el descriptor
como sus bytes.

| Descriptor | Layout x86_64 actual |
|---|---|
| buffer | `{ data: addr, len: u64, capacity: u64, flags: u64 }`, 32 bytes |
| slice | `{ data: addr, bytes: u64, flags: u64 }`, 24 bytes |
| span segment | `{ data: addr, bytes: u64 }`, 16 bytes |
| span | `{ segments: addr, count: u64, bytes: u64, flags: u64 }`, 32 bytes |

Los offsets se expresan en bytes. Null, desalineacion, rangos invalidos, UAF y
aliasing conservan comportamiento raw. `bufferCopy` es la unica operacion de la
familia que define solapamiento y actua como `memmove`.

## Materializacion

Los cuerpos `__ncc_buffer_*`, `__ncc_slice_*` y `__ncc_span_*` se generan dentro
del modulo LLVM/Machine. No importan `ncc_rt_*`, CRT ni un proveedor C.

Ejemplos ejecutables:

- `examples/runtime/families/memory_buffer_manual_raw.n`;
- `examples/runtime/families/memory_slice_basic.n`;
- `examples/runtime/families/memory_span_basic.n`;
- `examples/runtime/families/memory_views_manual_raw.n`.

El contrato histórico basado en `BufferHandle`, `SliceHandle`, `SpanHandle`,
`Result`, ownership y liberacion implicita esta retirado.
