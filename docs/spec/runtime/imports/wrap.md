# Import de runtime: `use <text:wrap>;`

## Contrato

`text:wrap` transforma texto UTF-8 mediante el proveedor manual nC de Unicode
17.0.0. La anchura se mide en columnas terminales y nunca se divide un grafema
extendido.

```n
use <text:wrap>;
```

| Funcion | Firma | Resultado |
|---|---|---|
| `wrap` | `wrap(value >> str, columns >> int) -> str` | Corte duro por columnas, respetando saltos existentes. |
| `wrapWords` | `wrapWords(value >> str, columns >> int) -> str` | Ajuste por palabras; una palabra demasiado ancha se divide por grafemas. |
| `indent` | `indent(value >> str, spaces >> int) -> str` | Anade espacios al principio de cada linea. |
| `wrapRelease` | `wrapRelease(value >> str) -> bool` | Libera una salida creada por esta familia. |

## Reglas Unicode

- los limites de grafema siguen UAX #29;
- la anchura reutiliza el contrato `text:width` para CJK, emoji, ZWJ,
  combining marks y selectores de variacion;
- `wrapWords` reconoce tabulador, espacio y los separadores Unicode
  configurados por el proveedor;
- NBSP, figure space y narrow no-break space no son puntos de corte;
- CR, LF, VT, FF, NEL, LS y PS conservan el salto duro de la entrada.

No se declara conformidad UAX #14 completa. El corte de palabras es una politica
determinista y explicita basada en grafemas, anchura terminal y espacios
separables.

## Memoria y errores

Las tres transformaciones reservan una cadena terminada en cero. El llamador es
dueno de esa salida y debe entregarla una vez a `wrapRelease`. Una columna no
positiva, una sangria negativa, UTF-8 invalido o un fallo de reserva devuelven
`null`. La entrada no se modifica ni se libera.

## Materializacion

Los simbolos canonicos `nC.runtime.wrap.*` bajan directamente a:

- `nccUnicodeRuntime_wrapRaw`;
- `nccUnicodeRuntime_wrapWordsRaw`;
- `nccUnicodeRuntime_indentRaw`;
- `nccUnicodeRuntime_wrapReleaseRaw`.

Las cuatro funciones estan escritas en `runtime/nc/unicode.n`. No se genera
fuente C ni se enlaza un runtime C para implementar el ajuste. El recorrido
certificado es:

```text
.n -> AST enlazado -> IR -> LLVM/MCT -> COFF .obj -> Machine Phase 12 -> PE64 .exe
```

El PE retenido solo importa `KERNEL32.dll`. Sus asignaciones raw se materializan
sobre memoria virtual del sistema y la copia de bytes se resuelve dentro del
enlace nativo.

## Ejemplo y evidencia

`examples/runtime/families/wrap_basic.n` cubre corte duro, palabras, CJK, una
familia emoji con ZWJ, combining marks, sangria y liberacion explicita. La
evidencia esta en
`target/evidence/imports/stoud-text-wrap-20260802-crtfree-final`.

Referencia normativa de segmentacion: [UAX #29](https://www.unicode.org/reports/tr29/).
