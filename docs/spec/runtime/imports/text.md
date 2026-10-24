# Import `use <text:text>;`

## Propósito

Superficie manual de texto por bytes. El import es explícito y su proveedor
está escrito en nC; no genera ni enlaza código C.

```n
use <text:text>;
```

## API

| Función | Retorno | Propiedad |
|---|---|---|
| `textLength(str)` | `int` | prestada |
| `textIsEmpty(str)` | `bool` | prestada |
| `textContains(str, str)` | `bool` | prestada |
| `textReplace(str, str, str)` | `str` | propia |
| `textTrim(str)` | `str` | propia |
| `textSplit(str, str)` | `array<str>` | propia |
| `textRelease(str)` | `bool` | destrucción manual |
| `textSplitRelease(p<byte>)` | `bool` | destrucción manual compuesta |

`textReplace` hace dos pasadas: cuenta coincidencias no solapadas, reserva una
sola vez y escribe bytes directamente. `textSplit` devuelve un descriptor
`data,len`; cada fragmento y la tabla se liberan con `textSplitRelease`.

No existen ownership, bounds checks, destructores ni crecimiento implícito.
UAF, doble liberación y punteros inválidos conservan semántica raw.

## Continuidad

```text
nC -> parser/sema -> IR -> proveedor nC -> LLVM/MCT -> .obj -> Machine -> .exe
```

ABI: `int=i32`, `bool=i1`, `str=ptr`, `array<str>={ptr,i64}`. El
objeto final no contiene símbolos `ncc_rt_text_*`, libc ni CRT de C.

## Evidencia

- `runtime/nc/text_format.n`
- `examples/runtime/families/text_basic.n`
- `crates/ncc-sema/tests/compact_text_text.rs`
- prueba IR `mct_llvm_lowers_manual_text_family_to_nc_provider_abi`
- prueba CLI `nc_text_provider_attaches_to_explicit_text_import`
- `target/evidence/imports/stoud-text-text-20260802-v1`
