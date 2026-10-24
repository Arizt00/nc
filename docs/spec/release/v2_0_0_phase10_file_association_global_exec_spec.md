# nC Phase10 File Association + Global Execution Spec v2.0.0

Estado: `frozen`  
Fase: `phase10_p12_file_association_global_exec`

## Objetivo

Cerrar F10-P12 de extremo a extremo con tres garantías productivas:

1. asociación editorial exclusiva para high-level (`.n`) y low-level soberano (`.nb/.nbb/.nm/.ncp`), manteniendo `.nc` solo como alias heredado del compilador y LSP;
2. ejecución global de `ncc` desde cualquier carpeta usando instalación `ncup`;
3. resolución de `runtime/stdlib` desde toolchain instalada (`~/.ncup/toolchains/<id>`), no desde el repo fuente.

## Asociación oficial de archivos

### VSCode (`nc-language`)

La extensión oficial DEBE registrar:

- `.n`
- `.nb`
- `.nbb`
- `.nm`
- `.ncp`

`.nc` NO DEBE ser reclamado por el plugin ni por su tema de iconos. El compilador y el LSP pueden seguir aceptándolo como alias heredado para migración.

### JetBrains (`nC Language Plugin`)

El plugin oficial DEBE registrar:

- `n`
- `nb`
- `nbb`
- `nm`
- `ncp`

`nc` NO DEBE registrarse como tipo de archivo JetBrains. Su compatibilidad queda limitada al compilador y al LSP.

## Ejecución global fuera del repo

`ncup init` DEBE:

- crear `NCUP_HOME/bin`;
- publicar shims de `ncc`/familia toolchain;
- permitir ejecutar `ncc` con `PATH` apuntando a `NCUP_HOME/bin` desde cualquier `cwd`.

Regla: el `cwd` NO DEBE depender del repo fuente para usar `ncc`.

## Resolución runtime/stdlib desde toolchain instalada

La toolchain instalada DEBE contener:

- `runtime_stdlib.v1.json`

`ncc stdlib-model` DEBE resolver primero desde instalación `ncup` activa y reportar origen:

- `resolution.source = toolchain_installation`

Fallback permitido:

- `resolution.source = embedded_fallback` cuando no exista contexto `NCUP_HOME` válido.

## Gate oficial

- `release_phase10_file_association_global_exec_gate_tests`

## Pipeline de verificación

- `ncc-ide-v43 ide43-association` ([build V43](../../NATIVE_TOOLS_V43_COMMANDS.md))
- `tools/release/qa_phase10_p12_v2_0_0.sh`

## Criterio de cierre

F10-P12 queda cerrado cuando:

1. contratos/documentos P12 están en `frozen`;
2. `.n`, `.nb`, `.nbb`, `.nm` y `.ncp` aparecen registrados en VSCode y JetBrains, mientras `.nc` queda sin asociación editorial;
3. `ncc` ejecuta desde carpeta externa al repo vía `NCUP_HOME/bin`;
4. `ncc stdlib-model --json` reporta resolución por toolchain instalada en entorno `ncup` válido;
5. reporte `F10_P12_file_association_global_exec_report.json` está en `ok`;
6. gate dedicado está en verde e integrado al release-train 2.x.
7. smoke fuera del repo valida el alias heredado `.nc` por compilador/LSP y `.nb/.nm/.ncp` con perfil `expert`, lane correcto y metadata editorial automática (`requested_editorial_profile` vs `editorial_profile_min`).
