# UUID nativo

## Superficie canonica

La familia `nC.runtime.uuid` forma parte del prelude hosted del backend. No
requiere `use`.

```n
bytes >> p<byte> <- stackAlloc(16, 16);
text >> p<byte> <- stackAlloc(37, 1);
written >> int <- uuidFromBytesInto(bytes, text, 37);
```

| Funcion | Firma | Propiedad |
|---|---|---|
| `uuidFromBytesInto` | `(p<byte>, p<byte>, u64) -> int` | Formatea 16 bytes en un buffer de 37 bytes |
| `uuidV4Into` | `(p<byte>, u64) -> int` | Obtiene entropia y escribe un UUID v4 canonico |
| `isUuid` | `(str) -> bool` | Valida la forma hexadecimal `8-4-4-4-12` |
| `uuidFromBytes` | `(array<byte,16>) -> str` | Compatibilidad asignadora |
| `uuidV4` | `() -> str` | Compatibilidad asignadora |
| `uuidRelease` | `(str) -> bool` | Libera un string creado por compatibilidad |

## Semantica raw

Las funciones `*Into` no asignan ni gestionan la vida del buffer. Devuelven 36
al escribir el texto y `-1` si el contrato material no puede completarse. La
capacidad minima es 37 bytes, incluido el terminador cero.

`uuidV4Into` modifica los bits de version y variante sobre 16 bytes obtenidos
del proveedor de entropia del sistema. `uuidFromBytesInto` conserva los bytes
aportados por el llamador.

Las funciones de compatibilidad poseen su almacenamiento hasta que el programa
llama expresamente a `uuidRelease`.

## Materializacion

LLVM/MCT define la conversion hexadecimal, los separadores, la validacion y la
gestion de compatibilidad. `uuidV4Into` enlaza directamente con el proveedor
`ncc_rt_entropy_fill` del target. No existe proveedor UUID escrito en C.

## Evidencia

- Ejemplo: `examples/runtime/families/io_uuid_basic.n`.
- Windows: parser, sema, IR, LLVM, objeto COFF, enlace PE y ejecucion nativa.
- La tabla de simbolos diferencia la unidad solo-entropia de la unidad UUID.
