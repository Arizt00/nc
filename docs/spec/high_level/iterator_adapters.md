# Iterator adapters

Status: closed with typed closures.

`map`, `filter` and `fold` accept a closure whose parameter and result types are checked against the iterator item type. Static callable targets lower to direct calls; captured closures use an explicit callable descriptor. No dynamic dispatch is inserted implicitly.

```n
@mapped >> list<int> : @items.map(|value: int| -> int {
    return @value + 1;
});
```

Wrong closure arity or types produce `E_HIGH_ITERATOR_ADAPTER_CLOSURE_001` or
`E_HIGH_ITERATOR_ADAPTER_TYPE_001`. The historical non-callable adapter spelling remains rejected.
