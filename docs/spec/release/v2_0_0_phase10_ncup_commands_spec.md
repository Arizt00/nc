# nC Phase10 ncup Mandatory Commands Spec v2.0.0

Estado: `frozen`  
Fase: `phase10_p5_ncup_commands`

## Objetivo

Cerrar el contrato operativo obligatorio de `ncup` para serie `2.x`, con comandos canónicos equivalentes al nivel de gestores maduros de toolchain.

## Base activo

Los siguientes comandos DEBEN existir y funcionar en `ncup`:

- `ncup install stable`
- `ncup default stable`
- `ncup toolchain list`
- `ncup toolchain install stable-x86_64-pc-windows-msvc`
- `ncup target add wasm32-wasi`
- `ncup component add nclsp`
- `ncup override set stable`
- `ncup update`
- `ncup self update`
- `ncup doctor`
- `ncup uninstall`

## Semántica mínima exigida

- `install` DEBE aceptar alias de canal (`stable|beta|nightly`) y resolver a toolchain-id canónico.
- `default` DEBE fijar toolchain por defecto y sincronizar proxies en `<home>/bin`.
- `toolchain list` DEBE enumerar toolchains instaladas y marcar default.
- `toolchain install` DEBE aceptar id completo de toolchain.
- `target add` DEBE registrar target para toolchain activa o explícita.
- `component add` DEBE registrar componente para toolchain activa o explícita.
- `override set` DEBE materializar `nc-toolchain.toml` en el proyecto.
- `update` DEBE refrescar solo la toolchain activa efectiva con transacción y rollback automático.
- `self update` DEBE dejar evidencia de actualización del gestor.
- `doctor` DEBE validar layout canónico y salud de bins.
- `uninstall` DEBE eliminar toolchain objetivo y reequilibrar default si aplica.

## Gate oficial

- `release_phase10_ncup_commands_gate_tests`

## Pipeline de verificación

- `tools/ncc/nc/src/verification/ncup_commands.n`
- `tools/release/qa_phase10_p5_v2_0_0.sh`

## Criterio de cierre

F10-P5 queda cerrado cuando:

1. contrato/spec quedan en estado `frozen`;
2. los 11 comandos obligatorios ejecutan sin ambigüedad;
3. la verificación end-to-end produce reporte `ok`;
4. el gate `release_phase10_ncup_commands_gate_tests` está en verde;
5. el suite `qa_phase10_v2_0_0.sh` incorpora F10-P5.
