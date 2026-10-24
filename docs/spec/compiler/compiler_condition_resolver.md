# Compiler Condition Resolver

## Coverage

The strict condition contract applies uniformly to `if`, `else if`, `while`, `do while`, classic `for`, `repeat until`, `each ... while`, `guard`, `unless`, `select/when`, ternary expressions, if-expressions, match guards, unary `!`, `&&`, and `||`.

## Type rule

A condition must have the exact type `bool` (or a union whose every member is `bool`). There is no implicit truthiness for `str`, numbers, chars, addresses, pointers, nullable values, collections, `Option`, `Result`, or unknown types. Sema reports `E_CONDITION_BOOL_REQUIRED_001`; nullable values retain `E_NULL_CONDITION_001`.

Strings remain valid operands of comparisons. For example, `if (name == "Ada")` is accepted because equality returns `bool`; `if (name)` is rejected.

## Phase continuity

The parser preserves the condition expression, sema proves its type, IR records a boolean branch input, the compiler audit emits the same diagnostic family, and the runtime executor refuses a non-`bool` value rather than coercing it.
