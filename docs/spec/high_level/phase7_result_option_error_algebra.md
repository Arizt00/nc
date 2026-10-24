# High Level Fase 7: Result / Option / Error Algebra

Status: closed.

Fase 7 closes the high-level source surface for explicit absence and explicit recoverable error transport. It adds official built-in algebraic data types `Option<T>` and `Result<T, E>` on top of the Fase 6 ADT and pattern matching foundation.

The phase is deliberately explicit:

- `Option<T>` represents presence or absence.
- `Result<T, E>` represents success or recoverable error.
- Absence and error are consumed with `match` or typed helper/method operations.
- `Some`, `None`, `Ok`, and `Err` are official constructors.
- `isSome`, `isNone`, `isOk`, and `isErr` are typed status helpers.
- `unwrap`, `unwrapOption`, `unwrapOr`, `unwrapOptionOr`, and `errorOf` preserve concrete payload types; non-fallback invalid variants fail at runtime.
- `?` and prefix `try` propagate `None`/`Err` and yield the success payload.
- Active `defer` blocks execute in LIFO order before every normal or propagated return.
- Exception syntax is rejected in the high-level profile.

Baseical example:

```n
Root {
    enum:
        ParseError {
            Empty;
            BadDigit;
        }

    parse :: in(flag >> bool) -> Result<int, ParseError> {
        if (flag) {
            return Ok(7);
        }
        return Err(ParseError.Empty);
    }

    valueOrZero :: in(flag >> bool) -> int {
        match (parse(flag)) {
            Ok(value) => return @value;
            Err(error) => return 0;
        }
    }
}
```

Closed gates:

- Option constructors and arity checks.
- Result constructors and arity checks.
- Contextual inference for `None`, `Ok`, and `Err`.
- Bidirectional inference through function parameters/returns, helpers, arrays, vectors, lists, queues, nested ADTs, and expression branches.
- Compact chained method calls are disambiguated against scoped values before container/module calls.
- Error type rejection for `any`.
- Option/Result match safety through Fase 6 exhaustiveness and unreachable-arm diagnostics.
- Explicit `match` and compact `?`/`try` propagation examples.
- CLI manifests: `high-level option`, `high-level result`, `high-level errors`.
- Doctor status: `high_level_phase7_result_option_error_algebra_closed`.

Outside this algebra closure:

- Iterators and ranges.
- Collection protocols.
- Async, effects, and concurrency.
- Closures and lambdas.
- Higher-kinded types.
- Specialization completion.
