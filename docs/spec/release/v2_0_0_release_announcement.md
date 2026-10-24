# nC v2.0.0 Announcement

nC v2.0.0 queda disponible como release formal de Fase 10.

## Qué incluye

- Cadena completa `ncup-init` + `ncup` + bins de toolchain.
- Integración IDE/LSP para JetBrains y VSCode.
- Seguridad de cadena de suministro con verificación obligatoria.
- Matriz de instalación/actualización/rollback validada.

## Para operadores

- Publicación externa y notas de release siguen runbooks canónicos en `tools/release/`.
- El compilador `ncc` mantiene autoridad semántica total del lenguaje.

## Próximos pasos

- Promoción de canal usando `tools/release/promote_release_train_v2_0_0.sh`.
- Publicación de artefactos por OS/canal usando el pipeline externo canónico.
