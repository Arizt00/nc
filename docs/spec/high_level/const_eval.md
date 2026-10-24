# Pure Const Eval

`const:` defines compile-time values for high-level source. A const declaration has a name, explicit type and initializer expression:

```n
const:
    answer: int = 40 + 2;
    title: str = "compile" + "-time";
```

The evaluator supports integers, booleans, strings, arrays/vectors, enum variants, public-safe TypeInfo values, indexing, arithmetic, comparisons and `if` expressions. Integer arithmetic is checked and reports overflow or division by zero.

Pure const eval is deliberately narrow. Calls that reach runtime IO, filesystem, process, system, FFI, machine, heap mutation, reflective mutation, dynamic eval, private metadata, construction leaks, side effects or `noLimit` are rejected before they can affect the host.
