# Runtime Rich Output

## Base
`print` and `println` remain the official runtime output statements. Rich
output is built as `str` values through `nC.runtime.output` and then printed by
the normal output path. This keeps the diagnostic families such as `print.mem`,
`print.raw`, `print.inspect`, and `print.stable` separate from user-facing
terminal presentation.

## Import
```n
use stoud.io { output };
```

## Constructors
- `color(text: str, name: str) -> str`
- `colorBold(text: str, name: str) -> str`
- `bgColor(text: str, name: str) -> str`
- `style(text: str, style: str) -> str`
- `emoji(name: str) -> str`
- `progressBar(label: str, current: int, total: int, width: int) -> str`
- `spinnerFrame(label: str, frame: int) -> str`
- `table2(leftHeader: str, rightHeader: str, leftValue: str, rightValue: str) -> str`
- `table3(colA: str, colB: str, colC: str, valueA: str, valueB: str, valueC: str) -> str`

## Color Contract
Foreground colors include black, red, green, yellow/orange, blue, magenta/purple,
cyan, white, gray, and bright variants. Background colors use the same names via
`bgColor`. Styles include bold, dim, italic, underline, blink, inverse, and
strike. `NC_NO_COLOR=1|true|yes|on` disables ANSI and returns plain text.

Unknown colors or styles fail with:
- `E_RUNTIME_RICH_OUTPUT_COLOR_001`
- `E_RUNTIME_RICH_OUTPUT_STYLE_001`

## Emoji Contract
`emoji` maps names to terminal text symbols. Current names include rocket, ok,
check, success, warn, warning, error, fail, cross, info, spark, fire, hourglass,
gear, lock, unlock, package, table, and pointer. Unknown names fail with
`E_RUNTIME_RICH_OUTPUT_EMOJI_001`.

## Tables And Animations
`progressBar` emits a stable ASCII progress bar and validates `total > 0` and
`width in 1..=120`. `spinnerFrame` emits one deterministic spinner frame by
index. `table2` and `table3` emit ASCII bordered tables with two or three
columns. These helpers are deterministic and suitable for golden tests.

## Example
```n
use stoud.io { output };

demo[@runtime] {
    run :: in() -> bool {
        println(color("estado=ok", "green"));
        println("emoji=" + emoji("rocket"));
        println(progressBar("compile", 3, 4, 8));
        println(table2("key", "value", "mode", "runtime"));
        return true;
    }

    ref main ->> run;

    fa:
        return true;
}
```

## Non Goals
This surface does not add new `print.*` modes, does not authorize runtime
printing in machine/basal/primitive modes, and does not bypass explicit
`use stoud.io { output };`.
