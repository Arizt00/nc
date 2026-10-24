# nC Phase10 Disk Layout Compatibility Spec v2.0.0

Estado: `frozen`  
Serie: `2.x`

## Compatibilidad activa

- `manifests/toolchains.v1.json` y `manifests/ncup-config.v1.json` son rutas canónicas.
- Layout legacy (`state/`, `config/`) puede coexistir por compatibilidad, pero NO es ruta primaria.
- `ncup` DEBE migrar automáticamente manifests legacy a rutas canónicas cuando detecte estado previo.

## Reglas

1. Cambios de nombres de directorios canónicos requieren RFC.
2. Cambios de nombre de `nc-toolchain.toml` requieren RFC.
3. Nuevos subdirectorios auxiliares PUEDEN añadirse sin romper base.
4. El comportamiento de override por proyecto NO DEBE depender del IDE.
