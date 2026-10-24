# High Level Fase 11 Capture Modes

Status: closed for Fase 11.

Closures require explicit capture lists. Implicit capture is rejected so the callable boundary is
auditable.

Capture modes:

```n
captures(@value)       // by value
captures(ref @value)   // immutable borrow
captures(mut @state)   // mutable borrow
captures(move @owned)  // moved ownership
```

Rules:

- captured symbols must exist before the closure expression;
- private container symbols cannot be captured into public closure surfaces;
- duplicate captures are rejected;
- mutable writes require `mut` capture;
- `mut` capture requires `effects(state.write)`;
- moved values cannot be captured again;
- escaping closures cannot return borrowed local captures;
- `move` closures may return owned captures safely.

Primary diagnostics:

- `E_HIGH_CAPTURE_UNKNOWN_001`
- `E_HIGH_CAPTURE_UNINITIALIZED_001`
- `E_HIGH_CAPTURE_VISIBILITY_001`
- `E_HIGH_CAPTURE_ALIAS_001`
- `E_HIGH_CAPTURE_MUT_REQUIRED_001`
- `E_HIGH_CAPTURE_MOVED_001`
- `E_HIGH_CLOSURE_RETURN_BORROW_001`
