# nC Toolchain Complete Compatibility Spec v1.0.1

Estado: `frozen`  
Serie objetivo: `1.x`

## Objetivo

Fijar compatibilidad de la toolchain completa cerrada en Fase 5:

- UX CLI estable;
- help canónico;
- errores accionables;
- uso fuera del repo en instalación limpia.

## Regla principal

En `1.x` no se permiten cambios breaking en:

- sintaxis canónica de comandos `nfmt/nlint/nbuild/npkg/nide`;
- superficie de help (`--help` / `ncc help ...`);
- contrato de hints accionables en errores;
- coherencia de alias/bin dedicados.

## Cambios permitidos en `1.x`

- mejoras internas sin ruptura de interfaz;
- ampliaciones compatibles de texto de ayuda;
- mejoras de diagnóstico que preserven hint accionable.

## Cambios prohibidos en `1.x`

- retirar comandos oficiales de tooling;
- romper flags canónicos;
- devolver errores sin hint de ayuda;
- requerir cwd del repo para usar herramientas base.

## Evidencia machine-readable

- `docs/spec/release/v1_0_1_toolchain_complete_contract.json`

## Gate obligatorio

- `release_phase5_toolchain_complete_gate_tests`
