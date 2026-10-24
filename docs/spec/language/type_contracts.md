# nC `type{}` Contracts

`type{}` in nC is a contract system, not OO sugar.

It defines explicit clauses for:
- field shape and visibility (`public/private/internal/strictly`)
- layout (`binary/unbinary/stable/runtime`, `repr`, `align`, `endian`)
- bounds and invariants
- constructure/destructure
- copy/clone/move
- ownership/nullability
- drop/finalization
- ffi representation
- methods/associated/validate/debug/compare/hash/stable contracts

Inside `type:` blocks, clause labels are `TypeContractSection` and are not global container sections.
This avoids the old parser regression:
`section not allowed outside [@primitive] context`.

nC design goal:
- explicit, auditable contracts
- no hidden semantic fallback
- typed diagnostics with stable codes

Header forms accepted by parser/sema:
- `type: Name { ... }`
- `type: Name<T, U> { ... }`
- `type: Name enum { Variant; Variant(payload: int); ... }`
- `type: Name tagged union { Variant(payload: T); ... }`

Enum/tagged-union variants are represented as contract variants in AST and validated in sema.
