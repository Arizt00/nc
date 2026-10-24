# nC Runtime Console — F7 Final Suite (v2.0.0)

## Objetivo
Cerrar validacion end-to-end de consola sobre binario real (`ncc`) con contratos reproducibles de:
- canales (`stdout`/`stderr`);
- formato (espacio/salto);
- render por modo (`human/debug/inspect/stable`);
- control ANSI (`NC_NO_COLOR`).

## Fixtures oficiales F7
- `examples/runtime_console_f7/f7ConsoleSplit.n`
- `examples/runtime_console_f7/f7ConsoleSpacing.n`
- `examples/runtime_console_f7/f7ConsoleRenderModes.n`

## Suite golden oficial
Archivo:
- `crates/ncc-cli/tests/runtime_console_golden.rs`

Cobertura:
- `f7_stdout_stderr_split_is_official_without_color`
- `f7_stderr_keeps_ansi_prefixes_when_color_enabled`
- `f7_spacing_surface_output_is_exact`
- `f7_render_modes_output_contract_is_stable`

## Contrato de compatibilidad (v2.0.0)
- No romper en minor:
  - separación de canales de `print.err`/`print.wm` en `stderr`;
  - semántica de `print.sp`/`print.nl`/`println()`;
  - selección de render por familia `print`.
- Cambios permitidos en minor:
  - ampliación aditiva de builtins;
  - nuevos formatos opcionales sin romper salida ya validada.
- Cambios breaking (solo major):
  - mover `print.err` o `print.wm` a `stdout`;
  - cambiar contrato textual golden actual de F7;
  - alterar resolución de modo de render de `print.dbg`/`print.inspect`/`print.sta`/`print.trc`.

## Comando de validación
```bash
cargo test -p ncc-cli --test runtime_console_golden -- --nocapture
```
