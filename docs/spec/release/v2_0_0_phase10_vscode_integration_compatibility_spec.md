# nC Phase10 VSCode Integration Compatibility Spec v2.0.0

Estado: `frozen`  
Serie: `2.x`

## Compatibilidad estructural

1. `ncc` DEBE seguir siendo autoridad semántica.
2. VSCode extension NO DEBE redefinir reglas de parser/sema/IR.
3. `nide` y `nc-sys-lsp` DEBEN operar como superficies editoriales complementarias.

## Compatibilidad de routing

- high-level DEBE seguir `broker-nide` por defecto.
- low-level DEBE seguir `lsp-direct` con `nc-sys-lsp`.
- cambios de routing por defecto REQUIEREN RFC.

## Compatibilidad de settings y comandos

La extensión DEBE conservar estos settings:

- `nc.toolchain`
- `nc.profile`
- `nc.target`
- `nc.formatter`

La extensión DEBE conservar estos comandos:

- `nC: Run`
- `nC: Check`
- `nC: Format`
- `nC: Select Toolchain`

## Compatibilidad de autodetección

`ncup which ncc --json` DEBE mantenerse estable como contrato base de resolución.

## Gate obligatorio

`release_phase10_vscode_gate_tests` DEBE validar contrato/artifacts/report para F10-P11.
