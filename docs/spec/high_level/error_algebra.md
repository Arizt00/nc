# High Level Error Algebra

Status: closed in Fase 7.

High-level recoverable errors are values. They are modeled with explicit enum types carried by `Result<T, E>`.

Error algebra rules:

- Error types should be named ADTs.
- Error values must be constructed explicitly.
- Error erasure to `any` is rejected.
- Error conversion is explicit; an `Err(SourceError.X)` is not silently accepted as `Err(TargetError.Y)`.
- Exception syntax is rejected.
- A function returning `Result<T, E>` must return `Ok(T)` or `Err(E)` through the type checker.

Explicit conversion example:

```n
Root {
    enum:
        SourceError { Missing; }
        TargetError { SourceMissing; }

    convert :: in(error >> SourceError) -> TargetError {
        match (error) {
            SourceError.Missing => return TargetError.SourceMissing;
        }
    }
}
```

Fase 7 does not add implicit conversion traits or effect rows. Those remain later-phase design space.
