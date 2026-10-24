# Runtime import: `nC.runtime.parse`

## 1. Purpose
`nC.runtime.parse` converts caller-provided text without libc, locale, hidden allocation or automatic cleanup. The implementation is ordinary nC compiled with the consumer.

## 2. Canonical compact import
```nC
use <text:parse>;
```

## 3. Aliases
`runtime.parse`

## 4. Mode availability
Allowed: `[@runtime]` examples and runtime-aware compiler analysis.

Denied: root and primitive under strict runtime import rules.

## 5. Exported functions
| Function | Signature | Return | Capability | Policy | Errors | Notes |
|---|---|---|---|---|---|---|
| `parseInt` | `parseInt(str) -> int` | `int` | `runtime.parse.basic, runtime.parse.strict, runtime.parse.try` | `imports.official-runtime-required, runtime.no-silent-fallback` | `E_RUNTIME_PARSE_ERROR_001, E_RUNTIME_PARSE_INT_ERROR_001, E_RUNTIME_PARSE_U_INT_ERROR_001, E_RUNTIME_PARSE_FLOAT_ERROR_001, E_RUNTIME_PARSE_BOOL_ERROR_001, E_RUNTIME_CONVERT_ERROR_001, E_RUNTIME_CONVERT_OVERFLOW_001, E_RUNTIME_VALIDATION_ERROR_001` | official export |
| `parseUInt` | `parseUInt(str) -> u64` | `u64` | `runtime.parse.basic, runtime.parse.strict, runtime.parse.try` | `imports.official-runtime-required, runtime.no-silent-fallback` | `E_RUNTIME_PARSE_ERROR_001, E_RUNTIME_PARSE_INT_ERROR_001, E_RUNTIME_PARSE_U_INT_ERROR_001, E_RUNTIME_PARSE_FLOAT_ERROR_001, E_RUNTIME_PARSE_BOOL_ERROR_001, E_RUNTIME_CONVERT_ERROR_001, E_RUNTIME_CONVERT_OVERFLOW_001, E_RUNTIME_VALIDATION_ERROR_001` | official export |
| `parseFloat` | `parseFloat(str) -> ft64` | `ft64` | `runtime.parse.basic, runtime.parse.strict, runtime.parse.try` | `imports.official-runtime-required, runtime.no-silent-fallback` | `E_RUNTIME_PARSE_ERROR_001, E_RUNTIME_PARSE_INT_ERROR_001, E_RUNTIME_PARSE_U_INT_ERROR_001, E_RUNTIME_PARSE_FLOAT_ERROR_001, E_RUNTIME_PARSE_BOOL_ERROR_001, E_RUNTIME_CONVERT_ERROR_001, E_RUNTIME_CONVERT_OVERFLOW_001, E_RUNTIME_VALIDATION_ERROR_001` | official export |
| `parseBool` | `parseBool(str) -> bool` | `bool` | `runtime.parse.basic, runtime.parse.strict, runtime.parse.try` | `imports.official-runtime-required, runtime.no-silent-fallback` | `E_RUNTIME_PARSE_ERROR_001, E_RUNTIME_PARSE_INT_ERROR_001, E_RUNTIME_PARSE_U_INT_ERROR_001, E_RUNTIME_PARSE_FLOAT_ERROR_001, E_RUNTIME_PARSE_BOOL_ERROR_001, E_RUNTIME_CONVERT_ERROR_001, E_RUNTIME_CONVERT_OVERFLOW_001, E_RUNTIME_VALIDATION_ERROR_001` | official export |
| `parseChar` | `parseChar(str) -> char` | `char` | `runtime.parse.basic, runtime.parse.strict, runtime.parse.try` | `imports.official-runtime-required, runtime.no-silent-fallback` | `E_RUNTIME_PARSE_ERROR_001, E_RUNTIME_PARSE_INT_ERROR_001, E_RUNTIME_PARSE_U_INT_ERROR_001, E_RUNTIME_PARSE_FLOAT_ERROR_001, E_RUNTIME_PARSE_BOOL_ERROR_001, E_RUNTIME_CONVERT_ERROR_001, E_RUNTIME_CONVERT_OVERFLOW_001, E_RUNTIME_VALIDATION_ERROR_001` | official export |
| `tryParseInt` | `tryParseInt(str, p<int>) -> bool` | `bool` | `runtime.parse.basic, runtime.parse.strict, runtime.parse.try` | `imports.official-runtime-required, runtime.no-silent-fallback` | `E_RUNTIME_PARSE_ERROR_001, E_RUNTIME_PARSE_INT_ERROR_001, E_RUNTIME_PARSE_U_INT_ERROR_001, E_RUNTIME_PARSE_FLOAT_ERROR_001, E_RUNTIME_PARSE_BOOL_ERROR_001, E_RUNTIME_CONVERT_ERROR_001, E_RUNTIME_CONVERT_OVERFLOW_001, E_RUNTIME_VALIDATION_ERROR_001` | caller-owned output |
| `tryParseFloat` | `tryParseFloat(str, p<ft64>) -> bool` | `bool` | `runtime.parse.basic, runtime.parse.strict, runtime.parse.try` | `imports.official-runtime-required, runtime.no-silent-fallback` | `E_RUNTIME_PARSE_ERROR_001, E_RUNTIME_PARSE_INT_ERROR_001, E_RUNTIME_PARSE_U_INT_ERROR_001, E_RUNTIME_PARSE_FLOAT_ERROR_001, E_RUNTIME_PARSE_BOOL_ERROR_001, E_RUNTIME_CONVERT_ERROR_001, E_RUNTIME_CONVERT_OVERFLOW_001, E_RUNTIME_VALIDATION_ERROR_001` | caller-owned output |
| `tryParseBool` | `tryParseBool(str, p<bool>) -> bool` | `bool` | `runtime.parse.basic, runtime.parse.strict, runtime.parse.try` | `imports.official-runtime-required, runtime.no-silent-fallback` | `E_RUNTIME_PARSE_ERROR_001, E_RUNTIME_PARSE_INT_ERROR_001, E_RUNTIME_PARSE_U_INT_ERROR_001, E_RUNTIME_PARSE_FLOAT_ERROR_001, E_RUNTIME_PARSE_BOOL_ERROR_001, E_RUNTIME_CONVERT_ERROR_001, E_RUNTIME_CONVERT_OVERFLOW_001, E_RUNTIME_VALIDATION_ERROR_001` | caller-owned output |

## 6. Type contracts
Function signatures are the active type contract surface for this import. Generic exports keep their declared generic marker in the function name.

## 7. Ownership rules
The caller owns every output slot passed to `tryParse*`. On failure the provider returns `false` and does not write the slot. Direct `parse*` forms return zero/false when input is invalid.

## 8. Nullability rules
Output pointers are raw. Passing null, stale, unaligned or out-of-range storage has the same undefined behavior as any manual raw pointer operation.

## 9. Bounds rules
Input is scanned byte by byte until NUL. The provider does not carry an implicit length or bounds object.

## 10. Side effects
Side effects are limited to the family purpose and guarded by the declared capabilities. IO, filesystem, process, network, security, and memory families require their listed policy gates.

## 11. Capabilities
- `runtime.parse.basic`
- `runtime.parse.strict`
- `runtime.parse.try`

## 12. Policies
- `imports.official-runtime-required`
- `runtime.no-silent-fallback`

## 13. Errors
- `E_RUNTIME_PARSE_ERROR_001` (RuntimeParseError)
- `E_RUNTIME_PARSE_INT_ERROR_001` (RuntimeParseIntError)
- `E_RUNTIME_PARSE_U_INT_ERROR_001` (RuntimeParseUIntError)
- `E_RUNTIME_PARSE_FLOAT_ERROR_001` (RuntimeParseFloatError)
- `E_RUNTIME_PARSE_BOOL_ERROR_001` (RuntimeParseBoolError)
- `E_RUNTIME_CONVERT_ERROR_001` (RuntimeConvertError)
- `E_RUNTIME_CONVERT_OVERFLOW_001` (RuntimeConvertOverflow)
- `E_RUNTIME_VALIDATION_ERROR_001` (RuntimeValidationError)

## 14. Positive examples
- `examples/runtime/families/parse_basic.n`

## 15. Negative examples
Negative missing-use behavior is verified by `crates/ncc-cli/tests/runtime_family_closure.rs` using the first exported symbol for each discovered family.

## 16. Strict mode
`ncc runtime check <file> --strict` accepts explicit runtime imports in `[@runtime]` and rejects runtime imports in forbidden modes.

## 17. Runtime report shape
The generated report for this family is stored at `reports/runtime_closure/runtime_reports/parse.json` and `reports/runtime_closure/runtime_reports/parse.md`.

## 18. Industrial conformance checklist
- import identity checked
- export signatures checked
- capabilities checked
- policies checked
- example check passed
- runtime JSON report generated
- runtime Markdown report generated

## 19. Known non-goals
This import does not make runtime calls available without `use`, does not bypass capability/policy checks, and does not cross into `[@machine]` or `primitive[@]`.

## 20. Evidence
- exports command: `ncc runtime exports nc.runtime.parse --json`
- check command: `ncc runtime check examples/runtime/families/parse_basic.n --strict --json`
- report command: `ncc runtime report examples/runtime/families/parse_basic.n --json`
- evidence hash: `da765dadd70d4b416de3b83d82f781e3012cac227af656627ac8c29568b91fa7`
