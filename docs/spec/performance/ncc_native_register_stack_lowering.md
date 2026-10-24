# nCC Native Register/Stack Lowering V1

Status: closed for scalar SysV64 register parameters, scalar stack locals and native-fast scalar returns.

This point moves native-fast beyond immediate return materialization. The backend can now lower selected scalar function bodies into real register and stack operations before `ret`.

## Closed Scope

- SysV64 scalar parameters are read from their ABI registers: `rdi`, `rsi`, `rdx`, `rcx`, `r8`, `r9`.
- Local scalar projections are assigned to frame stack slots.
- Scalar symbols can be loaded from parameters or stack locals.
- Arithmetic and bitwise scalar operations lower to x86_64 register instructions:
  - `+`, `-`, `*`
  - `/`, `%` through `cqo` and `idiv`
  - `&`, `|`, `^`
  - `<<`, `>>`, `>>>`
- Comparisons lower through `cmp`, `setcc`, and `movzx`.
- Local calls inside the same container are qualified to the emitted backend symbol.
- `ncc run --native-fast --show-return` executes the real backend bytes and reports the returned scalar.

## Examples

```n
nativeRegisterStackSumPerf[@] {
    add :: in(a >> int, b >> int) -> int {
        return a + b;
    }

    boot :: in() -> int {
        return add(20, 22);
    }

    ref main ->> boot;

    fa:
        return true;
}
```

The `add` function lowers to:

```text
mov rax, rdi
push rax
mov r10, rsi
pop rax
add rax, r10
ret
```

## Boundary

This is not a complete optimizing register allocator. Control-flow lowering, phi/SSA, global register allocation, loop lowering, arbitrary call ordering inside expression graphs, pointer/memRegion load-store lowering and direct non-hosted executable generation remain later work.

The closed guarantee is concrete: selected scalar functions no longer require constant folding to produce native return values.

## Evidence

Fixtures:

- `examples/performance/native_fast/register_stack_sum.n`
- `examples/performance/native_fast/register_stack_locals.n`

CLI smoke:

```text
ncc run examples/performance/native_fast/register_stack_sum.n --native-fast --show-return
ncc run examples/performance/native_fast/register_stack_locals.n --native-fast --show-return
```

Both must return:

```text
[return: 42 profile=release-fast route=native-fast-path]
```
