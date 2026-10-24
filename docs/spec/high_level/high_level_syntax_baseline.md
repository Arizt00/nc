# High Level Syntax Baseline

Status: closed for normal `.n` compact syntax.

This document records the normal source style for `.n` files. Compact syntax is
the official authoring surface for new normal/high-level nC code. Older verbose
forms remain accepted for compatibility and for low-level/migration fixtures,
but they are not the preferred style for new `.n` examples.

## Container Header

Baseical baseline form:

```n
entry[@] {
    ...
}
```

Other container forms are legacy/tolerated only when already accepted by the parser. They must not be used to claim a new phase closure.

## Official Compact Syntax

Preferred normal `.n` forms:

| Intent | Official compact form | Notes |
| --- | --- | --- |
| static/local declaration | `name >> type <- value;` | `>>` binds the declared type; `<-` performs first initialization. |
| typed declaration without init | `name >> type;` | Leaves material storage uninitialized. |
| assignment after declaration | `name = expr;` | Type is inferred from the declared symbol. |
| field assignment | `obj.field = expr;` | Equivalent to typed projection over the declared field. |
| function | `run :: in(arg >> Type) -> bool { ... }` | Canonical declaration; visibility may prefix the function explicitly. |
| type | `Packet :: type [repr C] { tag >> int; }` | Canonical aggregate and binary-layout declaration. |
| pointer read/write | `rd(p)` / `wr(p, value)` | Type is inferred from `p<T>`; no hidden checking is inserted. |
| address of | `&x` | Compact form of `addrOf(x)`. |
| pointer increment | `p++` | Advances by `sizeof(T)` for `ptr<T>`. |
| pointer post-increment read | `*p++` | C precedence: read old pointee, then advance pointer by `sizeof(T)`. |
| increment statement | `i++;` / `obj.field++;` | nC update statement over a mutable symbol or field. The normal authoring form is effect-first, not C-style ambiguous expression choreography. |
| decrement statement | `i--;` | Normalized as `i = i - 1` over the declared mutable target. |
| compound add/sub/mul | `i += n;` / `i -= n;` / `x *= n;` | Normalized as one target update using the existing typed projection path. |
| swap | `swap(a, b);` | Compact helper for simple symbol exchange. |

Compound operators are deterministic nC updates. They are not specified through
C's old-value/new-value sequence-point model: normal `.n` code should read them
as a single typed update of the target. `*p++` remains the explicit pointer
read-and-advance form because it names both effects in the operator itself.

Entrypoints stay intentionally small:

- `ref main ->> run;` is the official main reference for real programs.
- `to main { ... }` is for small scripts and teaching examples.

## Compatibility Forms

| Form | Status in Fase 1 | Notes |
| --- | --- | --- |
| `stc:` | official section | Static declarations in normal `.n` containers. |
| `fn:` | compatibility input | Parsed for old source; `nfmt` emits `name :: fn`. |
| `fa:` | official section | Final action/body section for normal `.n` containers. |
| `ref main ->> name;` | current baseline | Direct main function reference. |
| `use ...;` | current baseline | Import/use surface as already implemented. |
| `public:` / `private:` / `internal:` | current baseline | Visibility forms. |
| `>>` | current baseline | Projection/binding form where currently supported. |
| `!>` | current baseline | Strict check form where currently supported. |
| `@name >> type : value;` | compatibility verbose | Accepted as migration input; `nfmt` emits `name >> type <- value;`. |
| `@name` in ordinary expressions | compatibility verbose | Prefer `name` unless address/projection context requires `@`. |
| `dev(...)` | current baseline | Developer diagnostic/fault form where currently supported. |
| `return ...;` | current baseline | Current return form. |
| arrays/vectors/pointers | closed current baseline | Fixed arrays, real pointer forms and compact C-like pointer forms are part of normal `.n`. |

## Example

```n
entry {
    stc:
        a  >> int  <- 5;
        b  >> int  <- 10;
        ok >> bool <- false;

    run :: in() -> bool {
        swap(a, b);
        ok = a == 10 && b == 5;
        return ok;
    }

    ref main ->> run;

    fa:
        return true;
}
```

## Normalization Rule

Normal `.n` source should be written in compact syntax unless a test is
explicitly covering legacy/verbose compatibility, a lower modal layer, or a
diagnostic edge case.

The complete normative grammar is `docs/spec/language/canonical_compact_syntax_v3.md`.
