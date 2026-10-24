# Const Functions

`const fn:` declares functions callable by `const:` and `comptime:` pure declarations.

```n
const fn:
    add(a: int, b: int) -> int {
        public:
            return @a + @b;
    }

const:
    total: int = add(19, 23);
```

Const functions preserve the normal high-level parameter and return syntax, but their body is checked against the pure compile-time subset. They may use local projections, `if`, exhaustive enum `match`, pure expression statements and `return`. Runtime printing, loops without a bounded model, mutation, effects and calls to non-const functions are rejected.

Recursion is detected and bounded by the phase model. A recursive const function must fail with `E_HIGH_CONST_EVAL_RECURSION_LIMIT_001`; it cannot silently consume host resources.
