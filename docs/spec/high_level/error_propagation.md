# High-Level Error Propagation

Status: implemented end to end.

`?` and prefix `try` unwrap the success payload of `Option<T>` or
`Result<T,E>`. `None` or `Err(E)` returns the original tagged value from the
current function after executing every active `defer` in reverse registration
order.

```n
relay(enabled: bool) -> Result<int, str> {
    public:
        defer {
            release();
        }
        value >> int = source(enabled)?;
        return ok(value + 1);
}
```

Rules:

- `Option<T>?` is valid only in a function returning `Option<U>`.
- `Result<T,E>?` is valid only in a function returning `Result<U,E>`.
- The error channel must match exactly; implicit error conversion is rejected.
- `?` is postfix and does not conflict with the ternary `condition ? a : b`.
- `try value` has the same propagation semantics as `value?`.
- Propagation is forbidden in compile-time expressions.
- A `defer` body cannot contain `return`, `break`, or `continue`.
- Normal returns, propagated returns and fallthrough all run active cleanup.
- `defer` belongs to the current function or closure activation. Leaving a
  nested block with `break` or `continue` keeps its registered cleanup active;
  cleanup runs only when that activation returns, propagates or falls through.
- A closure has its own return contract and cleanup stack. Its `?` cannot use
  the enclosing function's `Option` or `Result` contract.
- An `async fn` completes its body and cleanup before publishing a ready
  future in the interpreter. Cancelling that future prevents a later await;
  it never skips cleanup that has already run.

Primary diagnostics:

- `E_HIGH_TRY_OPERAND_001`
- `E_HIGH_TRY_CONTEXT_001`
- `E_HIGH_DEFER_CONTROL_FLOW_001`

Materialization:

- AST: `UnaryOp::TryPropagate` and structured `Stmt::Defer`.
- IR: `ScalarUnaryOpIR::TryPropagate` and `StatementIR::Defer`.
- Interpreter: an internal propagation signal retains the original tagged value.
- LLVM and native-fast: tag test, payload extraction, cleanup epilogue and early return.
- Native AOT: async bodies are eager; `Future<T>` has the material layout of
  `T` and high-level `await` is an identity operation. The explicitly
  qualified runtime API retains its handle-based cancellation contract.
- Kernel profile: the same typed IR is compiled freestanding for every
  admitted kernel target; no hosted exception or unwinder is required.

Conformance fixtures:

- `examples/03_types/errorPropagationAdvanced.n` covers nested blocks,
  `break`, `continue`, closure invocation, async/await and propagated cleanup.
- `examples/11_machine/kernelErrorPropagation.n` covers freestanding
  `Option<T>` propagation and cleanup across the full kernel target matrix.
