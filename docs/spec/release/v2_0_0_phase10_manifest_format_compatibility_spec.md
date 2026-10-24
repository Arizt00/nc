# nC Phase10 Manifest Format Compatibility Spec v2.0.0

Estado: `frozen`  
Serie: `2.x`

## Compatibilidad activa

- `channel-manifest.toml` DEBE mantenerse estable en ruta y shape para serie `2.x`.
- `release-manifest.json` DEBE mantenerse estable para tooling externo y CI.
- `min_ncup_version` y `breaking_notes` DEBEN seguir presentes en ambos manifests.

## Reglas

1. Cambios breaking del shape base requieren RFC.
2. Campos nuevos PUEDEN añadirse de forma aditiva.
3. Campos obligatorios de F10-P8 NO DEBEN eliminarse en `2.x`.
4. La validación de hashes NO DEBE degradarse a best-effort.
