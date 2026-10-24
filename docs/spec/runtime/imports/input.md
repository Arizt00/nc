# nC Runtime Import: nc.runtime.input

## 1. Purpose
`nc.runtime.input` is the registered runtime import surface for the `input` family. It exposes operational functions under explicit import, capability, policy, and diagnostic contracts.

## 2. Canonical compact import
```n
use stoud.io { input };
```

## 3. Aliases
`runtime.input`

## 4. Mode availability
Allowed: `[@runtime]` examples and runtime-aware compiler analysis.

Denied: root and primitive under strict runtime import rules; machine and kernel for input.

## 5. Exported functions
| Function | Signature | Return | Capability | Policy | Errors | Notes |
|---|---|---|---|---|---|---|
| `readLine` | `readLine() -> str` | `str` | `runtime.input.read, runtime.input.tryRead, runtime.input.clear` | `imports.official-runtime-required, operators.bool-condition-required, operators.strict-types, runtime.no-silent-fallback` | `E_RUNTIME_INPUT_UNAVAILABLE_001, E_RUNTIME_INPUT_PARSE_ERROR_001, E_RUNTIME_INPUT_E_O_F_001, E_RUNTIME_INPUT_TOO_LONG_001, E_RUNTIME_INPUT_ENCODING_ERROR_001, E_RUNTIME_INPUT_CLOSED_001, E_RUNTIME_INPUT_INTERRUPTED_001` | official export |
| `readInt` | `readInt() -> int` | `int` | `runtime.input.read, runtime.input.tryRead, runtime.input.clear` | `imports.official-runtime-required, operators.bool-condition-required, operators.strict-types, runtime.no-silent-fallback` | `E_RUNTIME_INPUT_UNAVAILABLE_001, E_RUNTIME_INPUT_PARSE_ERROR_001, E_RUNTIME_INPUT_E_O_F_001, E_RUNTIME_INPUT_TOO_LONG_001, E_RUNTIME_INPUT_ENCODING_ERROR_001, E_RUNTIME_INPUT_CLOSED_001, E_RUNTIME_INPUT_INTERRUPTED_001` | official export |
| `readFloat` | `readFloat() -> ft` | `ft` | `runtime.input.read, runtime.input.tryRead, runtime.input.clear` | `imports.official-runtime-required, operators.bool-condition-required, operators.strict-types, runtime.no-silent-fallback` | `E_RUNTIME_INPUT_UNAVAILABLE_001, E_RUNTIME_INPUT_PARSE_ERROR_001, E_RUNTIME_INPUT_E_O_F_001, E_RUNTIME_INPUT_TOO_LONG_001, E_RUNTIME_INPUT_ENCODING_ERROR_001, E_RUNTIME_INPUT_CLOSED_001, E_RUNTIME_INPUT_INTERRUPTED_001` | official export |
| `readBool` | `readBool() -> bool` | `bool` | `runtime.input.read, runtime.input.tryRead, runtime.input.clear` | `imports.official-runtime-required, operators.bool-condition-required, operators.strict-types, runtime.no-silent-fallback` | `E_RUNTIME_INPUT_UNAVAILABLE_001, E_RUNTIME_INPUT_PARSE_ERROR_001, E_RUNTIME_INPUT_E_O_F_001, E_RUNTIME_INPUT_TOO_LONG_001, E_RUNTIME_INPUT_ENCODING_ERROR_001, E_RUNTIME_INPUT_CLOSED_001, E_RUNTIME_INPUT_INTERRUPTED_001` | official export |
| `readChar` | `readChar() -> char` | `char` | `runtime.input.read, runtime.input.tryRead, runtime.input.clear` | `imports.official-runtime-required, operators.bool-condition-required, operators.strict-types, runtime.no-silent-fallback` | `E_RUNTIME_INPUT_UNAVAILABLE_001, E_RUNTIME_INPUT_PARSE_ERROR_001, E_RUNTIME_INPUT_E_O_F_001, E_RUNTIME_INPUT_TOO_LONG_001, E_RUNTIME_INPUT_ENCODING_ERROR_001, E_RUNTIME_INPUT_CLOSED_001, E_RUNTIME_INPUT_INTERRUPTED_001` | official export |
| `tryReadLine` | `tryReadLine() -> result` | `result` | `runtime.input.read, runtime.input.tryRead, runtime.input.clear` | `imports.official-runtime-required, operators.bool-condition-required, operators.strict-types, runtime.no-silent-fallback` | `E_RUNTIME_INPUT_UNAVAILABLE_001, E_RUNTIME_INPUT_PARSE_ERROR_001, E_RUNTIME_INPUT_E_O_F_001, E_RUNTIME_INPUT_TOO_LONG_001, E_RUNTIME_INPUT_ENCODING_ERROR_001, E_RUNTIME_INPUT_CLOSED_001, E_RUNTIME_INPUT_INTERRUPTED_001` | official export |
| `tryReadInt` | `tryReadInt() -> result` | `result` | `runtime.input.read, runtime.input.tryRead, runtime.input.clear` | `imports.official-runtime-required, operators.bool-condition-required, operators.strict-types, runtime.no-silent-fallback` | `E_RUNTIME_INPUT_UNAVAILABLE_001, E_RUNTIME_INPUT_PARSE_ERROR_001, E_RUNTIME_INPUT_E_O_F_001, E_RUNTIME_INPUT_TOO_LONG_001, E_RUNTIME_INPUT_ENCODING_ERROR_001, E_RUNTIME_INPUT_CLOSED_001, E_RUNTIME_INPUT_INTERRUPTED_001` | official export |
| `tryReadFloat` | `tryReadFloat() -> result` | `result` | `runtime.input.read, runtime.input.tryRead, runtime.input.clear` | `imports.official-runtime-required, operators.bool-condition-required, operators.strict-types, runtime.no-silent-fallback` | `E_RUNTIME_INPUT_UNAVAILABLE_001, E_RUNTIME_INPUT_PARSE_ERROR_001, E_RUNTIME_INPUT_E_O_F_001, E_RUNTIME_INPUT_TOO_LONG_001, E_RUNTIME_INPUT_ENCODING_ERROR_001, E_RUNTIME_INPUT_CLOSED_001, E_RUNTIME_INPUT_INTERRUPTED_001` | official export |
| `tryReadBool` | `tryReadBool() -> result` | `result` | `runtime.input.read, runtime.input.tryRead, runtime.input.clear` | `imports.official-runtime-required, operators.bool-condition-required, operators.strict-types, runtime.no-silent-fallback` | `E_RUNTIME_INPUT_UNAVAILABLE_001, E_RUNTIME_INPUT_PARSE_ERROR_001, E_RUNTIME_INPUT_E_O_F_001, E_RUNTIME_INPUT_TOO_LONG_001, E_RUNTIME_INPUT_ENCODING_ERROR_001, E_RUNTIME_INPUT_CLOSED_001, E_RUNTIME_INPUT_INTERRUPTED_001` | official export |
| `clearLine` | `clearLine() -> bool` | `bool` | `runtime.input.read, runtime.input.tryRead, runtime.input.clear` | `imports.official-runtime-required, operators.bool-condition-required, operators.strict-types, runtime.no-silent-fallback` | `E_RUNTIME_INPUT_UNAVAILABLE_001, E_RUNTIME_INPUT_PARSE_ERROR_001, E_RUNTIME_INPUT_E_O_F_001, E_RUNTIME_INPUT_TOO_LONG_001, E_RUNTIME_INPUT_ENCODING_ERROR_001, E_RUNTIME_INPUT_CLOSED_001, E_RUNTIME_INPUT_INTERRUPTED_001` | official export |
| `hasInput` | `hasInput() -> bool` | `bool` | `runtime.input.read, runtime.input.tryRead, runtime.input.clear` | `imports.official-runtime-required, operators.bool-condition-required, operators.strict-types, runtime.no-silent-fallback` | `E_RUNTIME_INPUT_UNAVAILABLE_001, E_RUNTIME_INPUT_PARSE_ERROR_001, E_RUNTIME_INPUT_E_O_F_001, E_RUNTIME_INPUT_TOO_LONG_001, E_RUNTIME_INPUT_ENCODING_ERROR_001, E_RUNTIME_INPUT_CLOSED_001, E_RUNTIME_INPUT_INTERRUPTED_001` | official export |
| `windowReadKey` | `windowReadKey(handle: int) -> int` | `int` | `runtime.input.read` | `imports.official-runtime-required, runtime.no-silent-fallback` | `E_RUNTIME_WINDOW_PLATFORM_001` | Removes one `WM_KEYDOWN`, translates it, and returns its virtual-key code or zero. |
| `windowReadChar` | `windowReadChar(handle: int) -> int` | `int` | `runtime.input.read` | `imports.official-runtime-required, runtime.no-silent-fallback` | `E_RUNTIME_WINDOW_PLATFORM_001` | Removes one generated `WM_CHAR` and returns its UTF-16 code unit or zero. |

## 6. Type contracts
Function signatures are the active type contract surface for this import. Generic exports keep their declared generic marker in the function name.

## 7. Ownership rules
Ownership follows the declared argument and return contracts. Memory-bearing families additionally require origin, bounds, alias, or lifecycle policies when listed below.

## 8. Nullability rules
Nullability is explicit through `Option`, nullable pointer contracts, or family-specific refinement policies. Silent null widening is not part of this import contract.

## 9. Bounds rules
Bounds-sensitive operations are governed by the required capabilities and policies listed below. Out-of-range or invalid state must surface a official runtime diagnostic.

## 10. Side effects
Side effects are limited to the family purpose and guarded by the declared capabilities. IO, filesystem, process, network, security, and memory families require their listed policy gates.

## 11. Capabilities
- `runtime.input.read`
- `runtime.input.tryRead`
- `runtime.input.clear`

## 12. Policies
- `imports.official-runtime-required`
- `operators.bool-condition-required`
- `operators.strict-types`
- `runtime.no-silent-fallback`

## 13. Errors
- `E_RUNTIME_INPUT_UNAVAILABLE_001` (RuntimeInputUnavailable)
- `E_RUNTIME_INPUT_PARSE_ERROR_001` (RuntimeInputParseError)
- `E_RUNTIME_INPUT_E_O_F_001` (RuntimeInputEOF)
- `E_RUNTIME_INPUT_TOO_LONG_001` (RuntimeInputTooLong)
- `E_RUNTIME_INPUT_ENCODING_ERROR_001` (RuntimeInputEncodingError)
- `E_RUNTIME_INPUT_CLOSED_001` (RuntimeInputClosed)
- `E_RUNTIME_INPUT_INTERRUPTED_001` (RuntimeInputInterrupted)

## 14. Positive examples
- `examples/runtime/families/input_basic.n`

## 15. Negative examples
Negative missing-use behavior is verified by `crates/ncc-cli/tests/runtime_family_closure.rs` using the first exported symbol for each discovered family.

## 16. Strict mode
`ncc runtime check <file> --strict` accepts explicit runtime imports in `[@runtime]` and rejects runtime imports in forbidden modes.

## 17. Runtime report shape
The generated report for this family is stored at `reports/runtime_closure/runtime_reports/input.json` and `reports/runtime_closure/runtime_reports/input.md`.

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
- exports command: `ncc runtime exports nc.runtime.input --json`
- check command: `ncc runtime check examples/runtime/families/input_basic.n --strict --json`
- report command: `ncc runtime report examples/runtime/families/input_basic.n --json`
- evidence hash: `c4aba27c0a24c4dc29f5b10f88884b3dd54c378c085932e3dd0d74d2ee77239f`

## 21. Prompt safety
Every `read*` operation flushes pending stdout before it consumes stdin. This makes explicit prompt sequences and defensive auto-flush equivalent for ordering:

```n
print("> ");
flush();
@opcion >> int : readInt();
```

When `ncc run` receives piped stdin, consumed lines are echoed after the visible prompt to preserve the same logical transcript a terminal would show. Parse failures use `E_RUNTIME_INPUT_PARSE_001`; end of input uses `E_RUNTIME_INPUT_EOF_001`.

Native window input is queue-oriented rather than stdin-oriented.
`windowReadKey` calls `TranslateMessage` before returning, so a following
`windowReadChar` can consume the corresponding character without conflating
virtual-key codes with text. Neither function is injected into `[@machine]`;
both require the hosted runtime import/facade contract above.
