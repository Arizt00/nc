# nC Phase10 CI/CD Compatibility Spec (Compatibility) v2.0.0

Estado: `frozen`  
Serie: `2.x`

## Compatibilidad de instalación

1. El flujo CI DEBE mantener instalación por `channel + target + components`.
2. El setup oficial DEBE mantenerse compatible con `ncup` sin forks de comportamiento.

## Compatibilidad de cache

1. El cache key DEBE continuar ligado al hash SHA-256 de manifest.
2. Cambios de estructura de cache REQUIEREN RFC si rompen restauración determinista.

## Compatibilidad offline

1. El modo offline DEBE aceptar artifacts predescargados como fuente de bootstrap CI.
2. Offline NO DEBE requerir red para completar la instalación cuando el bundle está presente.

## Compatibilidad de integración

1. `nclsp` y `nc-sys-lsp` siguen siendo componentes de toolchain, no autoridades semánticas.
2. Action GitHub y template GitLab DEBEN permanecer como integración oficial de referencia.

## Gate obligatorio

`release_phase10_cicd_compat_gate_tests` DEBE validar este contrato.
