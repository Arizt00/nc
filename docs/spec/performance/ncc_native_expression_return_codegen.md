# nCC Native Expression Return Codegen

Status: closed for foldable scalar expression returns in the native-fast backend.

This point extends native scalar return codegen beyond literal-only returns. The backend can now preserve simple scalar expressions that are fully decidable during lowering and materialize the resulting value directly in the native return register.

## Scope

- `return (1 + 2) * 3;` lowers to `mov rax, 9 ; scalar-return` followed by `ret`.
- Local scalar projections with foldable values are tracked in function order.
- `@a >> int : 11; @b >> int : @a + 31; return @b;` lowers to `mov rax, 42 ; scalar-return`.
- Supported foldable values include numeric literals, bool literals, local scalar symbols, unary scalar operations and binary scalar operations already supported by the internal constant folder.
- The implementation preserves the legacy textual IR path for expressions that are not foldable.
- The emitted ELF object keeps the existing `.note.GNU-stack` non-executable stack contract.
- `ncc run --native-fast --show-return` reports the value returned by real native backend bytes.

## Boundary

This is not full register allocation. Native register/stack lowering for selected scalar runtime values is covered by `ncc_native_register_stack_lowering.md`. Dynamic memory loads, pointer loads, full SSA, arbitrary control-flow lowering, LTO and PGO remain later performance/compiler work.

The closed guarantee is narrower and real: if a scalar return can be proven from literals and local scalar projections during lowering, native-fast emits the computed immediate into `rax`.

## Evidence

Fixtures:

- `examples/performance/native_fast/optimizer_fold_dead.n`
- `examples/performance/native_fast/return_local_expr.n`

Expected backend behavior:

```text
return (1 + 2) * 3;        -> mov rax, 9; ret
@a=11; @b=@a+31; return @b -> mov rax, 42; ret
```

CLI smoke:

```text
ncc optimize examples/performance/native_fast/return_local_expr.n --json
ncc run examples/performance/native_fast/return_local_expr.n --native-fast --show-return
```

The optimize report must show `text_bytes = 8`. The run output must include:

```text
[return: 42 profile=release-fast route=native-fast-path]
```
