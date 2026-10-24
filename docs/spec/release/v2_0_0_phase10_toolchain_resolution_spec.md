# nC Phase10 Toolchain Resolution Spec v2.0.0

Estado: `frozen`  
Serie: `2.x`  
Fase: `phase10_p6_toolchain_resolution`

## Objetivo

Cerrar la resolucion determinista de toolchain para `ncup` de extremo a extremo, con diagnosticos accionables para faltantes de componentes y para incompatibilidades de tier/canal.

## Base activo

Orden obligatorio de resolucion implicita (sin `--toolchain` explicito):

1. override local (`nc-toolchain.toml` en proyecto)
2. variable de entorno (`NCUP_TOOLCHAIN`)
3. default global (`toolchains.v1.json`)

Reglas:

- `--toolchain <id>` EXPLICTO tiene precedencia sobre resolucion implicita.
- `nc-toolchain.toml` DEBE validar id baseico y compatibilidad tier/canal.
- `NCUP_TOOLCHAIN` DEBE validar id baseico y compatibilidad tier/canal.
- si no hay default global, DEBE fallar con error claro.

## Archivo de proyecto baseico

`nc-toolchain.toml` DEBE soportar:

- `[toolchain] id = "<channel>-<target>"`
- `[toolchain] channel = "<stable|beta|nightly>"` + `target = "<triple>"`

Parser/sema de `ncup` DEBE rechazar configuraciones ambiguas o incompletas con diagnostico accionable.

## Faltante de componente requerido

`ncup toolchain which --require-component <name>` DEBE:

- resolver toolchain con el orden baseico;
- validar existencia del componente en el modelo;
- fallar si no esta instalado en esa toolchain con mensaje accionable:
  - `missing required component '<name>'`
  - sugerencia: `ncup component add <name> --toolchain <id>`

## Incompatibilidad tier/canal

Si un target no soporta un canal para su tier:

- `ncup toolchain install <channel-target>` DEBE fallar con diagnostico de tier;
- `ncup target add <target>` sobre toolchain incompatible DEBE fallar igual.

Diagnostico minimo esperado:

- referencia al target y al canal rechazado;
- canales soportados para ese tier;
- workaround baseico sugerido, por ejemplo:
  - `ncup toolchain install nightly-x86_64-unknown-none-elf`

## Integracion CLI

- `ncup toolchain which --json` DEBE exponer:
  - `toolchain_id`
  - `source` (`local_override|env_var|global_default|explicit`)
  - `source_detail`
  - `resolution_order`
  - metadatos de home/cwd/env

- `ncup target add` y `ncup component add` sin `--toolchain` DEBEN usar el mismo orden determinista.

## Gate oficial

- `release_phase10_toolchain_resolution_gate_tests`

## Pipeline oficial

- `tools/ncc/nc/src/verification/ncup_resolution.n`
- `tools/release/qa_phase10_p6_v2_0_0.sh`
- `tools/release/qa_phase10_v2_0_0.sh` (suite global)

## Criterio de cierre

F10-P6 se considera cerrado cuando:

1. spec/compat/contract estan en estado `frozen`;
2. el orden de resolucion es estable y testeado;
3. los errores por componente faltante son accionables;
4. los errores por tier/canal incluyen diagnostico + workaround;
5. gate y QA global quedan en verde.
