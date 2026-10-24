# Runtime Exports Signatures

## Modelo
Cada familia publica exports con:
- nombre de función,
- aridad mínima,
- tipos de argumentos,
- tipo de retorno,
- variadic flag.

## Inspección
- `ncc runtime exports nC.runtime.input`
- `ncc runtime exports nC.runtime.output`

## Ejemplo
`nC.runtime.input` expone `readInt`, `readLine`, `tryReadInt`, etc.
`nC.runtime.output` expone `print`, `println`, `write`, `flush` y los
constructores ricos `color`, `colorBold`, `bgColor`, `style`, `emoji`,
`progressBar`, `spinnerFrame`, `table2` y `table3`.

## Output rico
- `color(text: str, name: str) -> str`
- `colorBold(text: str, name: str) -> str`
- `bgColor(text: str, name: str) -> str`
- `style(text: str, style: str) -> str`
- `emoji(name: str) -> str`
- `progressBar(label: str, current: int, total: int, width: int) -> str`
- `spinnerFrame(label: str, frame: int) -> str`
- `table2(leftHeader: str, rightHeader: str, leftValue: str, rightValue: str) -> str`
- `table3(colA: str, colB: str, colC: str, valueA: str, valueB: str, valueC: str) -> str`

Estos exports devuelven `str`: se materializan por `print`, `println`, `write`
o `writeln`. No sustituyen a las familias `print.*` de diagnostico/inspect.
