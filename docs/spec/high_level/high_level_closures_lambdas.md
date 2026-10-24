# High Level Fase 11 Closures And Lambdas

Status: closed. The Phase 11 heading is historical.

Fase 11 closes source, semantic, and callable support for closures/lambdas in high-level `nC`.
Closures are explicit callable values whose type is represented as `fn(params...) -> return`.

Examples:

```n
@addOne >> fn(int) -> int : |x: int| -> int {
    return @x + 1;
};

@reader >> fn() -> int : || -> int captures(ref @base) {
    return @base;
};
```

Supported Fase 11 behavior:

- closure literals as expressions;
- no-parameter lambdas;
- closure values assigned to locals;
- closure calls through local callable values;
- closures passed as parameters;
- owned closure return when captures are moved;
- basic iterator adapters `map`, `filter`, and `fold` with closure arguments;
- explicit diagnostics for missing types, mismatched function types, and bad call arity/types.

Current continuation beyond the original Phase 11 scope:

- async closures lower to native coroutine state machines;
- generator closures use typed `yield` transitions and persistent captures;
- iterator adapters resolve typed closure calls without hidden dynamic dispatch.

Reflection, higher-kinded types and specialization are separate language facilities and do not limit executable closures.
