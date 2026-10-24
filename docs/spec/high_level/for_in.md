# High Level For-In

`for-in` iterates over finite, typed iterable values.

Examples:

```n
@items >> int[] : [1, 2, 3];
@sum >> int : 0;
for item in @items {
    @sum >> int : @sum + @item;
}
```

Accepted iterable surfaces:

- `list<T>` with the required runtime list imports.
- `array<T,N>` / vector syntax already present in high-level.
- `memRegion<T>` with explicit runtime region imports.
- protocol values whose surface is typed as iterable.

Rules:

- the item type must be known and must not be `any`.
- the iterable expression must be list/vector/array-like or protocol-lowered.
- structural mutation of the iterated symbol inside the loop is rejected.

Diagnostics:

- `E_HIGH_FOR_IN_ITERABLE_001`
- `E_HIGH_FOR_IN_MUTATION_001`
- `E_HIGH_ANY_LEAK_001`
