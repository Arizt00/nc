# nC Pointer Compact C-Style Surface

This document records the compact pointer surface for normal nC runtime/high-level code. It is syntax sugar over the official pointer operations; it does not replace the backend model.

## Base

The long form remains official and explicit:

```n
ptrWrite<int>(@p, 7);
@value >> int : ptrRead<int>(@p);
@q >> ptr<int> : ptrOffset<int>(@p, 1);
@diff >> int : ptrDiff<int>(@q, @p);
@region >> memRegion<int> : regionFromPtr<int>(@p, 4);
@q >> ptr<int> : regionPtrAt<int>(@region, 0);
```

The compact form is accepted for C-style pointer work:

```n
@p <- 7;
@value >> int : @p.read();
@q >> ptr<int> : @p.offset(1);
@diff >> int : @q.diff(@p);
@region >> memRegion<int> : @p[..4];
@q >> ptr<int> : @region[0].ptr;
```

The compiler lowers the compact forms to the same runtime/backend operations:

| Compact form | Baseical operation |
| --- | --- |
| `@p <- v` | `ptrWrite<T>(@p, v)` |
| `@p.read()` | `ptrRead<T>(@p)` with raw direct unwrap |
| `@p.offset(n)` | `ptrOffset<T>(@p, n)` |
| `@q.diff(@p)` | `ptrDiff<T>(@q, @p)` |
| `@p[..n]` | `regionFromPtr<T>(@p, n)` |
| `@region[i].ptr` / `.ptr()` | `regionPtrAt<T>(@region, i)` |

`T` is inferred from `ptr<T>` or `memRegion<T>` when the receiver is typed. If no better type is known, the runtime fallback remains `int`, matching the existing pointer runtime compatibility lane.

## Safety And Diagnostics

The compact surface is intentionally raw and concise. It does not ask for a separate risk declaration. The backend still keeps the same checks and diagnostics when it can prove a problem:

- `E_PTR_TYPE_002` for applying pointer compact methods to non-pointer values.
- `E_PTR_NULL_DEREF_001` for null dereference when detected.
- `E_PTR_STALE_SITE_001` for stale heap/site provenance.
- `E_PTR_OUT_OF_BOUNDS_001` and `E_REGION_BOUNDS_001` for bounds failures.
- `E_PTR_ALIGNMENT_001` for invalid pointer distance/alignment.

The programmer keeps C/C++-grade control, while nC keeps the operation visible to the parser, checker, runtime, lowering reports and later evidence.

## Example

```n
use stoud.memory { ptr, heap, region };

ptrCompactCStyleOk[@runtime] {
    stc:
        p >> p<int>;
        q >> p<int>;
        view >> memRegion<int>;
        a >> int;
        b >> int;

    boot :: in() -> bool {
        p !> p<int> : alloc(32);
        wr(p, 7);

        view = p[..4];
        q = view[0].ptr;

        a = q.read();
        wr(q, 21);
        b = p.read();

        free(p);
        return a == 7 && b == 21;
    }

    ref main ->> boot;

    fa:
        return true;
}
```
