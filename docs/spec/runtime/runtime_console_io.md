# Runtime Console IO

The runtime console surface is ordered and materialized:

- `print(x)` writes `x` without a newline.
- `println(x)` writes `x` with a newline and flushes stdout.
- `flush()` forces pending stdout out before input.
- `readInt`, `readLine`, `readBool`, and `readChar` flush stdout before reading.

`ncc run` treats piped stdin as a transcript source: after a prompt is visible, the consumed input line is echoed in stdout so prompt evidence does not collapse into later prompts.

## Output Materialization
Console output accepts materialized runtime values. It rejects void output with `E_RUNTIME_OUTPUT_NONE_001`; parse failures from input are reported with `E_RUNTIME_INPUT_PARSE_001`.

## Import Discipline
In `[@runtime]`, console calls are explicit:

```n
use stoud.io { output, render, input };
```

`renderInt` remains the required path for typed string composition:

```n
println("Leido: " + renderInt(@x));
```
