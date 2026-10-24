# nC Runtime Import: nc.runtime.output

## 1. Purpose
`nc.runtime.output` is the registered runtime import surface for the `output` family. It exposes operational functions under explicit import, capability, policy, and diagnostic contracts.

## 2. Canonical compact import
```n
use stoud.io { output };
```

## 3. Aliases
`runtime.output`

## 4. Mode availability
Allowed: `[@runtime]` examples and runtime-aware compiler analysis.

Denied: root and primitive under strict runtime import rules.

## 5. Exported functions
| Function | Signature | Return | Capability | Policy | Errors | Notes |
|---|---|---|---|---|---|---|
| `print` | `print(any) -> void` | `void` | `runtime.output.write, runtime.output.flush, runtime.output.stable` | `imports.official-runtime-required, runtime.error.typed-required, runtime.no-secret-output, runtime.no-silent-fallback, runtime.redact-secrets` | `E_RUNTIME_OUTPUT_UNAVAILABLE_001, E_RUNTIME_OUTPUT_WRITE_ERROR_001, E_RUNTIME_OUTPUT_FLUSH_ERROR_001, E_RUNTIME_OUTPUT_ENCODING_ERROR_001, E_RUNTIME_OUTPUT_CLOSED_001, E_RUNTIME_OUTPUT_SECRET_FORBIDDEN_001` | official export |
| `println` | `println(any) -> void` | `void` | `runtime.output.write, runtime.output.flush, runtime.output.stable` | `imports.official-runtime-required, runtime.error.typed-required, runtime.no-secret-output, runtime.no-silent-fallback, runtime.redact-secrets` | `E_RUNTIME_OUTPUT_UNAVAILABLE_001, E_RUNTIME_OUTPUT_WRITE_ERROR_001, E_RUNTIME_OUTPUT_FLUSH_ERROR_001, E_RUNTIME_OUTPUT_ENCODING_ERROR_001, E_RUNTIME_OUTPUT_CLOSED_001, E_RUNTIME_OUTPUT_SECRET_FORBIDDEN_001` | official export |
| `println` | `println() -> void` | `void` | `runtime.output.write, runtime.output.flush, runtime.output.stable` | `imports.official-runtime-required, runtime.error.typed-required, runtime.no-secret-output, runtime.no-silent-fallback, runtime.redact-secrets` | `E_RUNTIME_OUTPUT_UNAVAILABLE_001, E_RUNTIME_OUTPUT_WRITE_ERROR_001, E_RUNTIME_OUTPUT_FLUSH_ERROR_001, E_RUNTIME_OUTPUT_ENCODING_ERROR_001, E_RUNTIME_OUTPUT_CLOSED_001, E_RUNTIME_OUTPUT_SECRET_FORBIDDEN_001` | official export |
| `write` | `write(str) -> result` | `result` | `runtime.output.write, runtime.output.flush, runtime.output.stable` | `imports.official-runtime-required, runtime.error.typed-required, runtime.no-secret-output, runtime.no-silent-fallback, runtime.redact-secrets` | `E_RUNTIME_OUTPUT_UNAVAILABLE_001, E_RUNTIME_OUTPUT_WRITE_ERROR_001, E_RUNTIME_OUTPUT_FLUSH_ERROR_001, E_RUNTIME_OUTPUT_ENCODING_ERROR_001, E_RUNTIME_OUTPUT_CLOSED_001, E_RUNTIME_OUTPUT_SECRET_FORBIDDEN_001` | official export |
| `writeln` | `writeln(str) -> result` | `result` | `runtime.output.write, runtime.output.flush, runtime.output.stable` | `imports.official-runtime-required, runtime.error.typed-required, runtime.no-secret-output, runtime.no-silent-fallback, runtime.redact-secrets` | `E_RUNTIME_OUTPUT_UNAVAILABLE_001, E_RUNTIME_OUTPUT_WRITE_ERROR_001, E_RUNTIME_OUTPUT_FLUSH_ERROR_001, E_RUNTIME_OUTPUT_ENCODING_ERROR_001, E_RUNTIME_OUTPUT_CLOSED_001, E_RUNTIME_OUTPUT_SECRET_FORBIDDEN_001` | official export |
| `flush` | `flush() -> bool` | `bool` | `runtime.output.write, runtime.output.flush, runtime.output.stable` | `imports.official-runtime-required, runtime.error.typed-required, runtime.no-secret-output, runtime.no-silent-fallback, runtime.redact-secrets` | `E_RUNTIME_OUTPUT_UNAVAILABLE_001, E_RUNTIME_OUTPUT_WRITE_ERROR_001, E_RUNTIME_OUTPUT_FLUSH_ERROR_001, E_RUNTIME_OUTPUT_ENCODING_ERROR_001, E_RUNTIME_OUTPUT_CLOSED_001, E_RUNTIME_OUTPUT_SECRET_FORBIDDEN_001` | official export |
| `color` | `color(str, str) -> str` | `str` | `runtime.output.write, runtime.output.rich` | `imports.official-runtime-required, runtime.no-silent-fallback, runtime.color-no-color-aware` | `E_RUNTIME_RICH_OUTPUT_TYPE_001, E_RUNTIME_RICH_OUTPUT_COLOR_001` | rich text constructor |
| `colorBold` | `colorBold(str, str) -> str` | `str` | `runtime.output.write, runtime.output.rich` | `imports.official-runtime-required, runtime.no-silent-fallback, runtime.color-no-color-aware` | `E_RUNTIME_RICH_OUTPUT_TYPE_001, E_RUNTIME_RICH_OUTPUT_COLOR_001` | rich text constructor |
| `bgColor` | `bgColor(str, str) -> str` | `str` | `runtime.output.write, runtime.output.rich` | `imports.official-runtime-required, runtime.no-silent-fallback, runtime.color-no-color-aware` | `E_RUNTIME_RICH_OUTPUT_TYPE_001, E_RUNTIME_RICH_OUTPUT_COLOR_001` | rich text constructor |
| `style` | `style(str, str) -> str` | `str` | `runtime.output.write, runtime.output.rich` | `imports.official-runtime-required, runtime.no-silent-fallback, runtime.color-no-color-aware` | `E_RUNTIME_RICH_OUTPUT_TYPE_001, E_RUNTIME_RICH_OUTPUT_STYLE_001` | rich text constructor |
| `emoji` | `emoji(str) -> str` | `str` | `runtime.output.write, runtime.output.rich` | `imports.official-runtime-required, runtime.no-silent-fallback` | `E_RUNTIME_RICH_OUTPUT_TYPE_001, E_RUNTIME_RICH_OUTPUT_EMOJI_001` | rich text constructor |
| `progressBar` | `progressBar(str, int, int, int) -> str` | `str` | `runtime.output.write, runtime.output.rich` | `imports.official-runtime-required, runtime.no-silent-fallback` | `E_RUNTIME_RICH_OUTPUT_TYPE_001, E_RUNTIME_RICH_OUTPUT_PROGRESS_001` | rich text constructor |
| `spinnerFrame` | `spinnerFrame(str, int) -> str` | `str` | `runtime.output.write, runtime.output.rich` | `imports.official-runtime-required, runtime.no-silent-fallback` | `E_RUNTIME_RICH_OUTPUT_TYPE_001` | rich text constructor |
| `table2` | `table2(str, str, str, str) -> str` | `str` | `runtime.output.write, runtime.output.rich` | `imports.official-runtime-required, runtime.no-silent-fallback` | `E_RUNTIME_RICH_OUTPUT_TYPE_001` | rich text constructor |
| `table3` | `table3(str, str, str, str, str, str) -> str` | `str` | `runtime.output.write, runtime.output.rich` | `imports.official-runtime-required, runtime.no-silent-fallback` | `E_RUNTIME_RICH_OUTPUT_TYPE_001` | rich text constructor |

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
- `runtime.output.write`
- `runtime.output.flush`
- `runtime.output.stable`

## 12. Policies
- `imports.official-runtime-required`
- `runtime.error.typed-required`
- `runtime.no-secret-output`
- `runtime.no-silent-fallback`
- `runtime.redact-secrets`

## 13. Errors
- `E_RUNTIME_OUTPUT_UNAVAILABLE_001` (RuntimeOutputUnavailable)
- `E_RUNTIME_OUTPUT_WRITE_ERROR_001` (RuntimeOutputWriteError)
- `E_RUNTIME_OUTPUT_FLUSH_ERROR_001` (RuntimeOutputFlushError)
- `E_RUNTIME_OUTPUT_ENCODING_ERROR_001` (RuntimeOutputEncodingError)
- `E_RUNTIME_OUTPUT_CLOSED_001` (RuntimeOutputClosed)
- `E_RUNTIME_OUTPUT_SECRET_FORBIDDEN_001` (RuntimeOutputSecretForbidden)

## 14. Positive examples
- `examples/runtime/families/output_basic.n`

## 15. Negative examples
Negative missing-use behavior is verified by `crates/ncc-cli/tests/runtime_family_closure.rs` using the first exported symbol for each discovered family.

## 16. Strict mode
`ncc runtime check <file> --strict` accepts explicit runtime imports in `[@runtime]` and rejects runtime imports in forbidden modes.

## 17. Runtime report shape
The generated report for this family is stored at `reports/runtime_closure/runtime_reports/output.json` and `reports/runtime_closure/runtime_reports/output.md`.

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
- exports command: `ncc runtime exports nc.runtime.output --json`
- check command: `ncc runtime check examples/runtime/families/output_basic.n --strict --json`
- report command: `ncc runtime report examples/runtime/families/output_basic.n --json`
- evidence hash: `9fdf3502642be2ef5a025a5925b2d4c5e518bffb826779cbfa046df1f5787bb2`

## 21. Console prompt ordering
`print` writes without a newline, `println` writes with a newline and flushes stdout, and `flush` forces pending stdout to be visible before any subsequent runtime input call. In `[@runtime]`, `print`, `println`, and `flush` require `use stoud.io { output };`.

The interactive prompt contract is:

```n
print("ID: ");
flush();
@id >> int : readInt();
```

`print` and `println` reject void output materialization with `E_RUNTIME_OUTPUT_NONE_001`. `flush` failures are reported as `E_RUNTIME_FLUSH_001`.

## 22. Rich output constructors
Rich output is value based. The helpers `color`, `colorBold`, `bgColor`,
`style`, `emoji`, `progressBar`, `spinnerFrame`, `table2`, and `table3`
return `str` values. They do not write to stdout by themselves and do not
replace `print`, `println`, or the diagnostic/inspect families `print.*`.

ANSI helpers honor `NC_NO_COLOR=1|true|yes|on` and degrade to plain text. Unknown
colors, styles, or emoji names fail with official runtime diagnostics instead
of being silently accepted.
