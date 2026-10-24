# Import de runtime: `use <text:width>;`

## Contrato

`text:width` mide texto UTF-8 mediante el proveedor manual nC de Unicode
17.0.0. No enlaza una biblioteca C ni delega la segmentacion o la anchura a la
plataforma.

```n
use <text:width>;
```

| Funcion | Firma | Resultado |
|---|---|---|
| `byteLength` | `byteLength(value >> str) -> int` | Bytes UTF-8 anteriores al terminador nulo. |
| `charCount` | `charCount(value >> str) -> int` | Valores escalares Unicode; `-1` si UTF-8 es invalido o falla la reserva. |
| `graphemeCount` | `graphemeCount(value >> str) -> int` | Grafemas extendidos segun UAX #29. |
| `displayWidth` | `displayWidth(value >> str) -> int` | Columnas visibles segun el contrato terminal descrito abajo. |
| `ansiVisibleWidth` | `ansiVisibleWidth(value >> str) -> int` | Columnas visibles despues de retirar secuencias ANSI CSI, OSC, DCS, SOS, PM y APC. |

## Anchura visible

La anchura terminal no procede unicamente de UAX #11. El contrato combina:

- propiedades `W` y `F` de `EastAsianWidth.txt` como dos columnas;
- segmentacion de grafemas extendidos de UAX #29;
- `Emoji_Presentation`, pictogramas extendidos, ZWJ, VS16 y keycaps;
- categorias Unicode sin anchura y `Default_Ignorable_Code_Point`;
- el resto de grafemas visibles como una columna.

Un cluster emoji visible ocupa dos columnas. Los clusters compuestos no suman
la anchura de sus marks, joiners o selectores. La politica para caracteres
ambiguos `A` es una columna; una futura politica locale debe ser explicita.

## Memoria y errores

Las funciones devuelven escalares y no transfieren memoria al llamador. El
proveedor usa memoria virtual raw temporal y la libera antes del retorno. Un
error de UTF-8 o de reserva produce `-1` en las funciones que decodifican. La
entrada sigue siendo una cadena terminada en cero y no se comprueba su region.

## Materializacion

Los simbolos canonicos `nC.runtime.width.*` bajan a funciones definidas en
`runtime/nc/unicode.n`; las tablas generadas estan en
`runtime/nc/unicode_tables.n`. El recorrido certificado es:

```text
.n -> AST enlazado -> IR -> LLVM/MCT -> COFF .obj -> Machine Phase 12 -> PE64 .exe
```

El generador consume datos oficiales de Unicode 17.0.0 y retiene sus hashes en
`runtime/unicode/17.0.0/MANIFEST.sha256`. La evidencia de cierre esta en
`target/evidence/imports/stoud-text-width-20260802-crtfree-final`.

## Ejemplo

`examples/runtime/families/width_basic.n` prueba UTF-8 multibyte, una familia
emoji con ZWJ, un grafema con combining mark, ideogramas y escapes ANSI.

Referencias normativas: [UAX #11](https://www.unicode.org/reports/tr11/) y
[UAX #29](https://www.unicode.org/reports/tr29/).
