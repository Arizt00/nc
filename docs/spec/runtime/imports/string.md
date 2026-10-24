# Import `use <text:string>;`

## Contrato

`text:string` es una biblioteca manual. El programa la declara de forma
explícita, igual que declararía una cabecera de sistema, pero su implementación
está escrita íntegramente en nC. No genera, compila ni enlaza código C.

```n
use <text:string>;
```

El compilador resuelve este import al proveedor
`runtime/nc/text_format.n`, lo incorpora al mismo AST enlazado y lo baja por:

```text
nC -> sema -> IR -> LLVM/MCT -> objeto del target -> Machine linker -> ejecutable
```

## Superficie

| Grupo | Funciones |
|---|---|
| Longitud | `strLen`, `len`, `strByteLen` |
| Construcción | `strEmpty`, `strConcat`, `repeatChar`, `charFromCode`, `charToStr` |
| Ventanas | `strSlice`, `strSubstring`, `substring` |
| Comparación | `strCompare`, `strEq`, `strStartsWith`, `strEndsWith` |
| Búsqueda | `strContains`, `strIndexOf` |
| Conversión | `strFromInt`, `strFromBool`, `strFromFt64` |
| Transformación | `strToUpper`, `strToLower`, `strTrim` |
| División | `strSplit`, `strSplitRelease` |
| Vida manual | `strRelease` |
| Carácter | `charCode` |

`strLen` y `strByteLen` recorren bytes terminados en cero. La familia Unicode
se mantiene separada para no ocultar decodificación, normalización ni memoria.
Las conversiones de mayúsculas/minúsculas de esta familia son ASCII.

## Memoria manual

Las operaciones que construyen texto devuelven almacenamiento propio:

- `strEmpty`, `strConcat`, `repeatChar`, `charFromCode`, `charToStr`;
- `strSlice`, `strSubstring`, `substring`;
- `strFromInt`, `strFromBool`, `strFromFt64`;
- `strToUpper`, `strToLower`, `strTrim`.

El llamador libera cada resultado exactamente cuando decida mediante
`strRelease`. El compilador no inserta destructores, ownership, comprobaciones
de vida ni limpieza implícita. Liberar literales, alias prestados, dos veces o
usar un valor después de liberarlo conserva la semántica raw del lenguaje.

`strSplit` asigna una tabla de punteros y un buffer por fragmento. Su descriptor
es `array<str>` (`data`, `len`) y se destruye explícitamente con:

```n
strSplitRelease(cast<byte>(&parts));
```

## ABI nativo

| Tipo nC | LLVM/MCT |
|---|---|
| `int` | `i32` |
| `u64`, `i64`, `addr` escalar | `i64` |
| `bool` | `i1` |
| `char`, `byte` | `i8` |
| `ft64` | `double` |
| `str`, `p<T>` | `ptr` |

El objeto final no importa símbolos `ncc_rt_str_*`, CRT de C ni libc para esta
familia. Solo puede importar primitivas del sistema operativo necesarias para
reservar/liberar páginas y escribir salida cuando el programa también usa
`io:output`.

## Ejemplo y evidencia

- Ejemplo: `examples/runtime/families/string_basic.n`.
- Proveedor: `runtime/nc/text_format.n`.
- Sema: `crates/ncc-sema/tests/compact_text_string.rs`.
- IR/ABI: prueba `mct_llvm_lowers_runtime_string_ops_to_host_bytes`.
- Compilador: prueba
  `nc_string_provider_is_manual_and_never_reclassified_as_hosted_c`.
- Ejecutable PE64: termina con código `0` y no enlaza un objeto runtime C.

Los límites de rango, null, doble liberación y UAF son responsabilidad del
programador. Una herramienta de auditoría puede diagnosticarlos, pero no cambia
el código generado.
