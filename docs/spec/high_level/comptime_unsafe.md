# Comptime Unsafe

`comptime unsafe:` is the high-level compile-time effect lane. It is not pure const eval and it does not grant ambient authority.

```n
comptime unsafe:
    readFixture {
        allow filesystem.read;
        filesystem.read("examples/high_level/phase13/data/read_allowed.txt");
    }
```

Every effect must have an explicit `allow`. Missing authority is a hard diagnostic. Effects record kind, capability, location, determinism, cacheability, detail and `noLimit` usage into an audit report. Effectful blocks disable pure caching unless they remain deterministic and cacheable by policy.

`comptime unsafe noLimit:` is reserved for declared unbounded loop or recursion authority and must include the corresponding `allow`.
