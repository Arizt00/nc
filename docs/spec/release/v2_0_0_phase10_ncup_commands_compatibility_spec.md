# nC Phase10 ncup Mandatory Commands Compatibility Spec v2.0.0

Estado: `frozen`  
Serie: `2.x`

## Compatibilidad activa

- Los comandos obligatorios de F10-P5 DEBEN mantenerse estables en la serie `2.x`.
- Los alias de canal (`stable|beta|nightly`) DEBEN seguir resolviendo a toolchain-id canónico.
- `ncup override set` DEBE seguir usando `nc-toolchain.toml` como archivo canónico.
- `ncup doctor` NO DEBE depender del IDE para validar layout o salud de bins.

## Reglas

1. Cambios breaking en sintaxis o semántica de comandos obligatorios requieren RFC.
2. Nuevos subcomandos PUEDEN añadirse sin romper los comandos existentes.
3. Nuevas opciones DEBEN ser aditivas y no alterar defaults canónicos sin RFC.
4. `ncup` PUEDE extender checks de `doctor`, pero NO DEBE degradar checks canónicos de F10-P4/F10-P5.
