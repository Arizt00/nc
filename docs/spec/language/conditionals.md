# Language Conditionals

## Surface

`if`, `else if`, `else`, `while`, `do while`, `for`, `repeat until`, `each ... while`, `guard`, `unless`, `select/when`, match guards, ternary expressions, and if-expressions.

## Rules

- Every condition is strictly `bool` in every normal/high-level/runtime profile.
- There is no implicit string, numeric, pointer, address, null, collection, `Option`, or `Result` truthiness.
- A string comparison is valid because `==`, `!=`, `<`, `<=`, `>`, and `>=` produce `bool` when their operands support that comparison.
- `match` requires coherent arms; typed ADT matches retain exhaustiveness and unreachable-arm checks.
- `guard` requires an explicit failure route.
