# Macro noLimit

Macro expansion is bounded by default:

- recursion depth: 8.
- expansion steps: 64.
- generated nodes: 256.
- nested expansions: 8.

The noLimit path is explicit and audited. It requires:

- `unsafe noLimit` macro declaration.
- `allow comptime.unbounded.loop;` or `allow comptime.unbounded.recursion;`.
- audit material through effect reporting when the macro performs an effect.

```n
macro:
    unsafe noLimit unlimited(name: ident) {
        allow comptime.unbounded.loop;
        allow comptime.sideEffects.report;
        effect sideEffects.report("noLimit audited");
        recursion 128;
        steps 512;
        generate fn name -> bool = true;
    }
```

noLimit without unsafe or without unbounded authority emits `E_HIGH_MACRO_NOLIMIT_001`.
