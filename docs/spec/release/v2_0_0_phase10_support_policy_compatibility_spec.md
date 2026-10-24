# nC Phase10 Support Policy Compatibility Spec v2.0.0

Estado: `frozen`  
Serie: `2.x`

## Compatibilidad de canales

1. `stable` DEBE mantener estado de soporte productivo completo.
2. `beta` DEBE mantener estado preview con migración documentada.
3. `nightly` DEBE mantener estado experimental sin SLA.

## Compatibilidad LTS

1. La ventana LTS DEBE permanecer explícita y versionada.
2. Cambios de ventana o alcance de backports REQUIEREN RFC.
3. El soporte enterprise DEBE permanecer acoplado a cortes LTS.

## Compatibilidad release-train

1. La política de soporte DEBE estar conectada al release train 2.x.
2. Cualquier promoción `nightly->beta->stable` DEBE preservar esta política.

## Gate obligatorio

`release_phase10_support_policy_gate_tests` DEBE validar este contrato.
