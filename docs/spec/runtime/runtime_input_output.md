# Runtime Input/Output

## Input
- `readLine`, `readInt`, `readFloat`, `readBool`, `readChar`
- `tryRead*` para rutas no fatales.

## Output
- `print`, `println`, `write`, `writeln`, `flush`
- control por policies de secreto/redacción.
- salida rica como valor `str`: `color`, `colorBold`, `bgColor`, `style`, `emoji`,
  `progressBar`, `spinnerFrame`, `table2`, `table3`.
- `print` y `println` siguen siendo la autoridad de emision; los helpers ricos no
  imprimen solos, solo construyen texto renderizable.

## Contrato
Sin import `nC.runtime.input/output`, los símbolos no resuelven.
`NC_NO_COLOR=1|true|yes|on` desactiva ANSI y preserva texto plano.
