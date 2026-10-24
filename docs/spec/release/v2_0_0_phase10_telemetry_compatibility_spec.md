# nC Phase10 Telemetry Spec (Compatibility) v2.0.0

Estado: `frozen`  
Serie: `2.x`

## Compatibilidad de política

1. La política `opt-in-only` DEBE mantenerse.
2. La telemetría deshabilitada por defecto DEBE mantenerse.
3. Cualquier cambio de política de consentimiento REQUIERE RFC.

## Compatibilidad de métricas mínimas

1. Los eventos de instalación DEBEN seguir incluyendo `status`, `ncup_version` y `target`.
2. La estructura de evento DEBE mantener `schema` y `version` explícitos.

## Compatibilidad de privacidad

1. `ncup` NO DEBE recolectar código fuente del usuario.
2. `ncup` NO DEBE recolectar contenido de archivos fuente.
3. Cualquier expansión de telemetría DEBE preservar esta prohibición.

## Gate obligatorio

`release_phase10_telemetry_gate_tests` DEBE validar este contrato.
