# Compiler Array Type Resolution (v2.1.0)

Implemented:
- parser support for generic const args (`array<int, 5>`).
- parser support for C-style lengths (`int[5]`) lowered to generic array form.
- type compatibility bridge between `array<T, N>`, `array<T>`, and vector literal projection surface.

Fixed in this phase:
- indexed lvalue projection route no longer fails with `method call on non-list type Array(...)`.
