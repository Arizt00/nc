# Tests Organization

Baseical test layout:

- `tests/compile-pass/`
- `tests/compile-fail/`
- `tests/fixtures/`
- `tests/runtime/`
- `tests/root/`
- `tests/machine/`
- `tests/primitive/`
- `tests/security/`
- `tests/network/`
- `tests/compiler/`
- `tests/tooling/`
- `tests/inspect/`
- `tests/backend/`

## Compatibility policy

Existing test entrypoints and fixtures remain unchanged in this cleanup phase.

Directory creation is complete; migration of test files into official buckets is a separate controlled pass that must include:

1. rust test harness path updates,
2. fixture path updates,
3. CI script updates,
4. golden-output verification.
