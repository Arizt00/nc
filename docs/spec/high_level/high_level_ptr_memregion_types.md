# High Level Ptr And MemRegion Types

Status: closed.

Pointer and memory-region forms:

- `p<T>` / `ptr<T>`;
- `ptr<T> | null`;
- `addr`;
- `memRegion<T>`.

Rules:

- `ptr<T>` and `addr` are distinct;
- `addr` accepts explicit numeric literals as address constants;
- an `int` variable is not an `addr`;
- projecting/casting `addr` to `p<T>` is accepted without ownership, provenance, lifetime, or risk authority;
- `memRegion<T>` requires runtime region/list authority;
- plain raw pointers may contain null; nullable unions remain available for API modeling;
- raw hazards compile with C-like undefined behavior, while `memRegion<T>` keeps its separate bounds contract.

Examples:

```n
@base >> addr : 0x1000;
@maybe >> ptr<byte> | null : null;
@region >> memRegion<int> : listAsRegion(@items);
```
