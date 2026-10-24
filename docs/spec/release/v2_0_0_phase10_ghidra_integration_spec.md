# nC Phase10 Ghidra Integration Spec v2.0.0

Estado: `frozen`  
Fase: `phase10_p20_ghidra_integration`

## Objetivo

Cerrar integración oficial Ghidra para nC 2.x con prioridad en low-level:

- `primitive[@]` como superficie principal;
- soporte secundario para `[@machine]` y `[@]`;
- routing lane-aware (`nc-sys-lsp` low-level, `nclsp` fallback high-level);
- extensión instalable `.zip` para Ghidra;
- gate y reporte machine-readable.

## Base activo

1. `ncc` DEBE mantener autoridad semántica.
2. `nc-sys-lsp` DEBE operar como asistencia low-level especializada.
3. Ghidra NO DEBE emitir reglas en conflicto con parser/sema/IR canónicos.

## Artefacto oficial

Ubicación canónica:

- `tools/ghidra/nc-ghidra-extension`

El paquete DEBE incluir:

- `extension.properties`
- `Module.manifest`
- `ghidra_scripts/NcPrimitiveAnalyze.java`
- `ncc-tools-v43 route-ghidra` (`tools/ncc/nc/src/verification/ide_router.n`)

## Routing canónico

- `.nb/.nm/.ncp` o encabezado soberano -> `nc-sys-lsp analyze --json`
- high-level (`.n/.nc`) -> `nclsp validate ... --ide ghidra --json`

## Gate oficial

- `release_phase10_ghidra_gate_tests`

## Pipeline de verificación

- `ncc-ide-v43 ide43-ghidra` ([build V43](../../NATIVE_TOOLS_V43_COMMANDS.md))
- `tools/release/qa_phase10_p20_v2_0_0.sh`

## Criterio de cierre

1. spec/compat/contract en `frozen`;
2. router Ghidra enruta low-level/high-level de forma determinista;
3. bundle Ghidra `.zip` generado con manifest y hash;
4. reporte `F10_P20_ghidra_integration_report.json` en `ok`.
