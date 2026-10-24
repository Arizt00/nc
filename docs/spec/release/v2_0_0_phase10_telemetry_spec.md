# nC Phase10 Telemetry Spec v2.0.0

Estado: `frozen`  
Fase: `phase10_p15_telemetry`

## Objetivo

Cerrar F10-P15 con telemetría de instalación **opt-in** y garantía de privacidad:

1. telemetría deshabilitada por defecto;
2. métricas mínimas obligatorias: éxito/fallo de instalación, versión, target;
3. prohibición explícita de recolectar código fuente del usuario.

## Superficie canónica

- `ncup telemetry status [--home <path>] [--json]`
- `ncup telemetry enable [--home <path>] [--json]`
- `ncup telemetry disable [--home <path>] [--json]`

## Reglas canónicas

1. La telemetría DEBE ser `opt-in-only`.
2. El estado por defecto DEBE ser `opt_in=false`.
3. `NCUP_TELEMETRY_OPT_IN` PUEDE sobrescribir el valor de configuración de forma explícita.
4. Los eventos DEBEN almacenarse en `logs/telemetry-events.v1.ndjson` dentro de `NCUP_HOME`.
5. La familia de eventos de instalación DEBE contener como mínimo:
- `status` (`success|failure`)
- `ncup_version`
- `target`
6. El sistema NUNCA DEBE recolectar:
- código fuente;
- contenido de archivos fuente;
- snippets de código del usuario.

## Gate oficial

- `release_phase10_telemetry_gate_tests`

## Pipeline de verificación

- `ncc-tools-v43 dist-telemetry` ([build V43](../../NATIVE_TOOLS_V43_COMMANDS.md))
- `tools/release/qa_phase10_p15_v2_0_0.sh`

## Criterio de cierre

F10-P15 queda cerrado cuando:

1. spec/compat/contract P15 están en `frozen`;
2. `ncup telemetry status` muestra `enabled=false` por defecto;
3. `enable/disable` aplica roundtrip determinista;
4. la instalación escribe eventos con campos mínimos obligatorios;
5. se evidencia evento de éxito y de fallo de instalación;
6. el reporte `docs/status/F10_P15_telemetry_report.json` queda en `ok`;
7. el gate P15 queda integrado en QA global y release-train 2.x.
