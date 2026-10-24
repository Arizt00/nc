# nC Phase10 JetBrains Integration Compatibility Spec v2.0.0

Estado: `frozen`  
Serie: `2.x`

## Compatibilidad activa

- `ncup which ncc` DEBE mantenerse estable como contrato de autodetección.
- `nclsp` DEBE conservar rol de backend general de integración JetBrains.
- `nc-sys-lsp` DEBE conservar rol opcional especializado low-level.
- El plugin oficial NO DEBE contradecir la autoridad semántica de `ncc`.

## Reglas

1. Cambios breaking en contrato de autodetección/ruteo requieren RFC.
2. Nuevas features editoriales PUEDEN añadirse de forma aditiva.
3. El router de modo DEBE seguir reglas de familia modal + extensión canónica.
4. En conflicto de diagnóstico, prioridad canónica: `ncc` > adaptadores editoriales.
