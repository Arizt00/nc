# Import `use <text:truncate>;`

Familia manual de truncado por bytes, implementada en nC.

```n
use <text:truncate>;
```

| Función | Semántica |
|---|---|
| `truncate(text, limit)` | primeros `limit` bytes |
| `truncateEnd(text, limit)` | prefijo y marcador `...` |
| `truncateMiddle(text, limit)` | conserva ambos extremos y añade `...` |
| `truncateRelease(text)` | libera un resultado propio |

El segundo argumento es `int`; se eliminó el contrato histórico `any`. Todas
las operaciones devuelven un bloque propio, incluso cuando el texto ya cabe.
Esto mantiene una única regla manual: cada resultado se destruye exactamente
una vez con `truncateRelease`.

No hay bounds checks, Unicode oculto, asignación automática posterior ni código
C. El límite se interpreta en bytes; la política Unicode pertenece a
`text:unicode`.

Evidencia:

- `runtime/nc/text_format.n`
- `examples/runtime/families/truncate_basic.n`
- `crates/ncc-sema/tests/compact_text_truncate.rs`
- prueba IR `mct_llvm_lowers_manual_truncate_family_to_nc_provider_abi`
- `target/evidence/imports/stoud-text-truncate-20260802-v1`
