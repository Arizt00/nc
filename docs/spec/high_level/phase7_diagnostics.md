# High Level Fase 7 Diagnostics

Status: closed.

Baseical diagnostics added or exercised by Fase 7:

- `E_HIGH_OPTION_ARITY_001`: `Option` requires exactly one type argument or matching constructor arity.
- `E_HIGH_OPTION_NONE_CONTEXT_001`: `None` was used where the expected type is not `Option<T>`.
- `E_HIGH_OPTION_UNKNOWN_001`: a helper received a non-Option or an inference hole escaped all valid contexts.
- `E_HIGH_OPTION_SOME_TYPE_001`: `Some(value)` does not match the expected payload type.
- `E_HIGH_OPTION_FALLBACK_TYPE_001`: an Option fallback does not unify with `T`.
- `E_HIGH_RESULT_ARITY_001`: `Result` requires exactly two type arguments or matching constructor arity.
- `E_HIGH_RESULT_OK_TYPE_001`: `Ok(value)` does not match the expected success type.
- `E_HIGH_RESULT_ERR_TYPE_001`: `Err(error)` does not match the expected error type.
- `E_HIGH_RESULT_FALLBACK_TYPE_001`: a Result fallback does not unify with `T`.
- `E_HIGH_RESULT_ERROR_MISMATCH_001`: a Result helper received an incompatible value.
- `E_HIGH_RESULT_ERROR_ANY_001`: a `Result` error channel cannot be `any`.
- `E_HIGH_ERROR_ERASURE_001`: error values cannot be erased into `any`.
- `E_HIGH_ANY_LEAK_001`: `any` cannot leak through Option/Result.
- `E_HIGH_TRY_OPERAND_001`: `?` or `try` requires an `Option<T>` or `Result<T,E>` operand.
- `E_HIGH_TRY_CONTEXT_001`: the current return contract cannot propagate the operand's absence/error channel.
- `E_HIGH_DEFER_CONTROL_FLOW_001`: deferred cleanup cannot escape with `return`, `break`, or `continue`.
- `E_HIGH_UNWRAP_DEFERRED_001`: legacy unqualified `unwrap(Option<T>)` is ambiguous; typed methods, `unwrapOption`, and Result helpers are closed.
- `E_HIGH_EXCEPTION_UNSUPPORTED_001`: exception-style calls are rejected in high-level error algebra.

Fase 6 match diagnostics remain active for Option/Result:

- `E_HIGH_MATCH_NON_EXHAUSTIVE_001`
- `E_HIGH_MATCH_UNREACHABLE_ARM_001`
- `E_HIGH_MATCH_ARM_TYPE_001`
