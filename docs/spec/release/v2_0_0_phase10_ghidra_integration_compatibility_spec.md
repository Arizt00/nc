# nC Phase10 Ghidra Integration Compatibility Spec v2.0.0

Estado: `frozen`

## Reglas de compatibilidad

1. El backend low-level de Ghidra DEBE seguir siendo `nc-sys-lsp`.
2. La extensión Ghidra NO DEBE sustituir autoridad semántica de `ncc`.
3. Cambios en router/contrato Ghidra que alteren selección modal REQUIEREN RFC.
4. `primitive[@]` DEBE conservar prioridad en lane Ghidra.

## Estabilidad de interfaz

La siguiente superficie DEBE mantenerse estable:

- `ncc-tools-v43 route-ghidra --file <path> --json`
- bundle `nc-ghidra-extension-v2_0_0.zip`
- manifest `ghidra_extension_manifest.v1.json`

El [builder V43](../../NATIVE_TOOLS_V43_COMMANDS.md) publica el basename
`ncc-tools-v43`; la migracion desde el nombre previo `ncc-tools` no cambia
el subcomando, los argumentos ni el contrato de routing/JSON.

## Política de ruptura

No se permiten cambios breaking sin RFC explícita y nota de migración.
