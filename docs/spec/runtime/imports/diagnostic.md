# Import `use <audit:diagnostic>`

This is the compatible raw audit-table API. Compiler, machine and IDE
diagnostics use the canonical v2 ABI documented in
`docs/spec/diagnostics/canonical_machine_diagnostic_abi_v2.md`.

## Contrato

`audit:diagnostic` almacena diagnosticos binarios en una tabla propiedad del
programa. No crea objetos, strings, JSON, spans administrados ni salida.

```n
use <audit:diagnostic>;
```

## Layout ABI

Cada entrada legacy ocupa 64 bytes: severidad, codigo, puntero y longitud del mensaje,
puntero de fuente, linea, columna y flags; cada campo reserva ocho bytes.

## API raw

`diagStride()` devuelve 64. `diagWrite(table, index, severity, code, message,
messageBytes, source, line, column, flags)` escribe una entrada. Las funciones
`diagSeverity`, `diagCode`, `diagMessage`, `diagMessageBytes`, `diagSource`,
`diagLine`, `diagColumn` y `diagFlags` leen campos. `diagCopy` copia un registro
byte a byte y `diagClear` lo pone a cero.

Todos los punteros son prestados. El programa conserva buffers, capacidad,
texto, codificacion, persistencia y salida. Los indices y direcciones invalidos
se materializan sin checks y tienen comportamiento indefinido.

## Materializacion

Las doce operaciones legacy bajan a aritmetica de direcciones, loads, stores y bucles
LLVM/Machine. No usan `malloc`, `memcpy`, JSON, logging, libc ni proveedor C.
El ejemplo real es `examples/runtime/families/audit_diagnostic_raw.n`.

Codigo nuevo que deba cruzar parser, machine, runtime, kernel, IDE o ghor debe
usar el registro canonico de 128 bytes. El formato de 64 bytes permanece solo
para no romper consumidores de auditoria existentes.
