# nC Phase10 First-Run Spec v2.0.0

Estado: `frozen`  
Fase: `phase10_p13_first_run`

## Objetivo

Cerrar F10-P13 de extremo a extremo para experiencia `first-run` productiva:

1. `ncup-init` pregunta perfil/canal/target cuando hay TTY interactiva;
2. bootstrap instala layout canónico + toolchain por defecto;
3. `ncup-init` ejecuta `ncup doctor` automáticamente;
4. `ncup-init` crea proyecto demo y valida `ncc run hello.n`;
5. `ncup-init` entrega guía corta post-instalación.

## Reglas canónicas

1. `ncup-init` DEBE soportar:
- `--default-toolchain <id>`
- `--channel <stable|beta|nightly>` + `--target <triple>`
- `--profile <minimal|default|complete>`
- `--yes|--non-interactive` para desactivar prompts
- `--skip-first-run` solo para escenarios controlados de automatización.

2. En TTY interactiva y sin `--yes`, `ncup-init` DEBE solicitar:
- perfil;
- canal;
- target por defecto.

3. `ncup-init` DEBE ejecutar `ncup doctor` al final del bootstrap y fallar si `doctor` falla.

4. `ncup-init` DEBE generar demo canónica:
- ruta: `<NCUP_HOME>/tmp/first-run-demo/hello.n`;
- validación: ejecución real de `ncc run hello.n`.

5. `ncup-init` DEBE producir salida machine-readable (`--json`) con bloque `first_run` que incluya:
- wizard usado/no usado;
- channel/target seleccionados;
- resultado de doctor;
- resultado de demo;
- guía post-instalación.

## Gate oficial

- `release_phase10_first_run_gate_tests`

## Pipeline de verificación

- `tools/ncc/nc/src/verification/ncup_first_run.n`
- `tools/release/qa_phase10_p13_v2_0_0.sh`

## Criterio de cierre

F10-P13 queda cerrado cuando:

1. spec/compat/contract P13 están en `frozen`;
2. `ncup-init --json` reporta `first_run.doctor.status = ok`;
3. `ncup-init --json` reporta `first_run.demo.status = ok`;
4. existe demo `hello.n` en `tmp/first-run-demo`;
5. el reporte `docs/status/F10_P13_first_run_report.json` está en `ok`;
6. gate dedicado está verde e integrado al release-train 2.x.
