# High Level Type System

Status: Fase 2 closed.

The high-level type system is the source-level contract that sits above basal and machine. Fase 2 closes the current advanced surface without starting complete generics, traits, pattern matching, closures, async, reflection, or property testing.

The closed surface covers:

- primitive identity for `bool`, `int`, `uint`, `ft`, `str`, `null`, and `void`;
- numeric width aliases and implicit narrowing rejection;
- nullability through `T | null`;
- structural union assignment and rejection of operations that require refinement;
- `array<T,N>`, `T[]`, `vector<T>`-family surfaces, `list<T>`, and `memRegion<T>`;
- `ptr<T>` and `addr` distinction with explicit literal address bridge only;
- function arity, parameter type, and return type checks;
- strict check operator `!>` as the current cast/coercion gate;
- limited inference for numeric literals and homogeneous collection literals.

Within that surface, `vector<S>` where `S` is a type-contract struct denotes a
dynamic material sequence backed by the same descriptor-and-buffer model as
`dinArray<S>`. Scalar `vector<T>` remains the storage/SIMD vector family.

Fase 2 does not make `any` a hidden escape hatch. Unknown compatibility remains only for legacy surfaces already present in the compiler and is documented as compatibility debt, not as a new type-system rule.

Baseical command:

```bash
ncc high-level types --json
```

The manifest emitted by that command is the machine-readable authority for the Fase 2 closure.
