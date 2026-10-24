# High Level Iterators

`Iterator<T>` is the finite pull protocol.

```n
next() -> Option<T>
```

`Some(T)` carries the next item and `None` represents exhaustion. The item type is never erased. `Iterator`, `Iterable`, `IntoIterator` and `Collection` require exactly one non-`any` type argument.

```n
consume(iter: Iterator<int>) -> bool {
    public:
        @next >> Option<int> : @iter.next();
        match (@next) {
            Some(value) => return @value >= 0;
            None => return true;
        }
}
```

`for-in` over fixed arrays, dynamic arrays, vectors, lists and regions lowers before backend emission to length, index and typed element reads. `map`, `filter` and `fold` require typed closures and are statically checked; untyped callable spellings are rejected.

Generators and streams use the resumable descriptor protocol documented in `iterators_generators_async.md`; they do not hide collection materialization or scheduler work.

Diagnostics include `E_HIGH_ITERATOR_PROTOCOL_001`, `E_HIGH_ITERATOR_PROTOCOL_ARITY_001`, `E_HIGH_ITERATOR_ITEM_001`, `E_HIGH_ITERATOR_NEXT_001`, `E_HIGH_ITERATOR_ADAPTER_CLOSURE_001`, `E_HIGH_ITERATOR_ADAPTER_TYPE_001` and `E_HIGH_ANY_LEAK_001`.
