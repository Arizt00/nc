# nC Phase10 Toolchain Resolution Compatibility Spec v2.0.0

Estado: `frozen`  
Serie: `2.x`

## Compatibilidad activa

- El orden `override local -> env var -> default global` DEBE permanecer estable en serie `2.x`.
- `nc-toolchain.toml` DEBE seguir siendo el archivo baseico de override por proyecto.
- `NCUP_TOOLCHAIN` DEBE conservar semantica de override de sesion.
- Los mensajes de componente faltante DEBEN mantener sugerencia accionable con `ncup component add`.
- Los mensajes de incompatibilidad tier/canal DEBEN mantener workaround explicito.

## Reglas

1. Cualquier cambio breaking en precedencia requiere RFC.
2. Nuevos metadatos en `--json` PUEDEN agregarse de forma aditiva.
3. Nuevos checks PUEDEN ampliarse, pero NO DEBEN romper contratos de F10-P6.
4. La resolucion implicita NO DEBE depender del IDE.
