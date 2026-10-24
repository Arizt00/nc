# nC DX + Observability Spec v1.0.1

Estado: `frozen`  
Serie: `1.x`  
Fase: `phase8_dx_observability`

## Objetivo

Cerrar DX y observabilidad de producción con tipado end-to-end para troubleshooting rápido:

- diagnósticos con códigos estables;
- sugerencias automáticas en errores de CLI/superficie;
- trazabilidad compilación/runtime;
- perfiles de ejecución JSON canónicos.

## Superficie canónica

- códigos estables mínimos:
  - `NCC-CLI-0001` errores accionables de sintaxis/uso;
  - `NCC-CLI-0002` comando desconocido;
  - `NCC-CLI-0003` ayuda desconocida;
  - familia `NCC-DX-xxxx` para fallos por etapa en `dx-profile`;
  - familia `NCC-LEX/PARSE/SEMA/RUNTIME` en reportes estructurados.
- sugerencias automáticas:
  - “did you mean …” para comandos/acciones cercanas.
- perfil de ejecución:
  - `ncc dx-profile <check|run> <file.n> ...`;
  - salida `ncc.dx.profile.v1` con etapas y tiempos.

## Contratos de observabilidad

- schema de perfil: `ncc.dx.profile.v1`.
- schema de diagnósticos IDE: `nide.diagnostics.v1` con `error_entries` tipadas.
- tiempos por etapa en ms (`read_source`, `lex`, `parse`, `source_conventions`, `sema`, `main_entry_validation`, `runtime`).
- reportes de error con `code`, `stage`, `message`, `hint`.

## DoD de Fase 8

Fase 8 se considera cerrada solo si:

1. spec + compat + contract están en `frozen`;
2. gate `release_phase8_dx_observability_gate_tests` está en verde;
3. perfiles `dx-profile` funcionan para `check` y `run` con salida estructurada reproducible;
4. códigos y sugerencias se mantienen estables en casos críticos;
5. debugging/troubleshooting típico se resuelve en minutos mediante reportes estructurados.

## Gate obligatorio

- `release_phase8_dx_observability_gate_tests`
