# nCC Native Scalar Return Codegen

Status: closed for immediate scalar returns in the native-fast backend.

This phase closes the first native semantic payload beyond a bare `ret`: functions that return a literal `int`/number or `bool` now lower into a backend `return_imm` operation and materialize real x86_64 bytes before the function returns.

## Scope

- `return 7;` lowers to `mov rax, 7 ; scalar-return` followed by `ret`.
- `return true;` lowers to immediate value `1`.
- `return false;` lowers to immediate value `0`.
- Decimal, hexadecimal, binary and underscored numeric literals are accepted through the existing backend numeric parser.
- The emitted ELF object keeps the existing `.note.GNU-stack` non-executable stack contract.
- Normal `ncc run --native-fast` keeps process success behavior by using the silent hosted launcher.
- `ncc run --native-fast --show-return` uses a capture launcher that calls the real backend symbol, reads the native return value, prints it, and exits successfully.

## Boundary

This point is literal-only. Foldable expression returns and local scalar projection returns are covered by `ncc_native_expression_return_codegen.md`. Runtime expression lowering, parameter lowering, register allocation, SSA, global optimization, LTO and PGO remain future performance/compiler work. The point closed here is narrower and real: the backend no longer loses simple scalar literal return semantics.

## Evidence

Fixture:

`examples/performance/native_fast/return_7.n`

Expected backend text:

```text
.fn nativeScalarReturnPerf.boot ; mode=root cc=nC.sysv64 frame=0B
nativeScalarReturnPerf.boot:
    mov rax, 7 ; scalar-return
    ret
```

Expected x86_64 bytes:

```text
48 C7 C0 07 00 00 00 C3
```

CLI smoke:

```text
ncc optimize examples/performance/native_fast/return_7.n --json
ncc run examples/performance/native_fast/return_7.n --native-fast --show-return
```

The optimize report must show `text_bytes = 8`. The run output must include:

```text
[return: 7 profile=release-fast route=native-fast-path]
```
