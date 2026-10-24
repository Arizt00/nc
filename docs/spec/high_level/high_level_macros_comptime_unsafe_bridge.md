# Macro comptime-unsafe bridge

Unsafe macros are the only macro family allowed to declare compile-time effects. They must be written as `unsafe` and must list each authority through `allow` clauses. Effects then use `effect capability("detail");`.

Supported authorities mirror Fase 13:

- filesystem read/write/list/create/delete/metadata.
- process spawn/status.
- system env/user/path.
- heap mutation.
- reflection mutation.
- dynamic eval.
- private metadata.
- construction leak.
- side effects report.
- unbounded loop or recursion under noLimit.

Example:

```n
macro:
    unsafe audited(name: ident) {
        allow comptime.dynamic.eval;
        effect dynamic.eval("checked by macro audit");
        generate fn name -> bool = true;
    }
    expand audited(dynamicGenerated);
```

Missing `allow` emits `E_HIGH_MACRO_UNDECLARED_EFFECT_001`. Declaring such effects outside the unsafe lane emits the matching macro diagnostic.
