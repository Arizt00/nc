# nC Toolchain Complete Spec v1.0.1

Estado: `frozen`  
Serie: `1.x`  
Fase: `phase5_toolchain_complete`

## Objetivo

Cerrar la toolchain completa de producción con tipado end-to-end:

- `ncc`
- `nfmt`
- `nlint`
- `nbuild`
- `npkg`
- `nide`

El cierre exige:

- CLI UX estable;
- help canónico por herramienta;
- errores accionables con hint de uso;
- coherencia operacional entre alias/bin dedicados;
- ejecución usable fuera del repo en instalación limpia.

## Superficie CLI canónica

- `ncc --help`
- `ncc help <command>`
- `<tool> --help`
- `ncc <tool> --help`

Comandos de tooling obligatorios:

- `nfmt <file.n> [--check|--compact|--in-place|--out <path>]`
- `nlint <file.n>`
- `nbuild <file.n> [out-exe] [--out-dir <dir>] [--json] [--cc <profile>] [--linker <bin>] [--entry <symbol>]`
- `npkg <manifest|check-imports|list-imports> <file.n> [--out <path>]`
- `nide <outline|symbols|diagnostics> <file.n> [--out <path>]`

## Política de error accionable

Toda falla de sintaxis/opción en herramientas debe incluir:

- causa concreta;
- hint explícito a `--help`;
- forma canónica del comando.

## DoD de Fase 5

Fase 5 se considera cerrada solo si:

1. spec + compat + contract están en `frozen`;
2. `release_phase5_toolchain_complete_gate_tests` está en verde;
3. herramientas son ejecutables desde directorio limpio con rutas absolutas (sin dependencia de cwd del repo);
4. contrato legacy referencia gate y contrato de fase 5.

## Gate obligatorio

- `release_phase5_toolchain_complete_gate_tests`
