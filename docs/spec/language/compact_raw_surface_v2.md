# Compact raw surface v2 (historica)

Esta superficie fue reemplazada por `canonical_compact_syntax_v3.md`. Se
conserva para explicar la migracion de fuentes antiguas y no define la escritura
oficial.

Status: implemented as source aliases over the canonical typed AST.

## Compatibility

No existing syntax is removed. Compact declarations and raw pointer aliases
normalize before semantic analysis, so IR, LLVM/MCT, machine and native object
generation retain one implementation.

## Declarations

```nc
value <- expression;
value: Type = expression;
```

`<-` explicitly requests compile-time inference and has no material runtime
effect. The typed form keeps every type decision in source. Manual code that
wants no inference uses the typed form. Existing `var`, `>>`, `!>` and verbose
local sections remain accepted.

## Raw pointer aliases

| Compact | Canonical AST call |
| --- | --- |
| `rd<T>(p)` | `ptrRead<T>(p)` |
| `wr<T>(p, value)` | `ptrWrite<T>(p, value)` |
| `off<T>(p, elements)` | `ptrOffset<T>(p, elements)` |
| `dist(left, right)` | `ptrDiff(left, right)` |
| `nil(p)` | `ptrIsNull(p)` |
| `adr(p)` | `ptrAddr(p)` |
| `from<T>(address)` | `ptrFromAddr<T>(address)` |
| `asptr<T>(p)` | `ptrCast<T>(p)` |

These calls do not add bounds, alignment, lifetime, ownership, provenance or
overflow checks. Invalid addresses, out-of-range offsets, aliasing, UAF and
double free compile under the normal manual/raw contract. Their observable
result is undefined, as with equivalent raw C operations.

The optional pointer auditor may report hazards when explicitly invoked. It
never changes normal sema or generated code.
