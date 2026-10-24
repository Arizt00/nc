# nC Phase10 First-Run Compatibility Spec v2.0.0

Estado: `frozen`  
Serie: `2.x`

## Compatibilidad funcional

1. `ncup-init` DEBE mantener `first-run` como flujo oficial de bootstrap.
2. La validación automática `ncup doctor` DEBE mantenerse activa por defecto.
3. La validación de demo `ncc run hello.n` DEBE mantenerse activa por defecto.

## Compatibilidad de UX

1. El modo interactivo DEBE existir cuando hay TTY.
2. La automatización DEBE preservarse con `--yes|--non-interactive`.
3. Cambios en prompts NO DEBEN romper `--json` ni CI.

## Compatibilidad de contrato

1. El bloque `first_run` en salida JSON DEBE mantenerse estable.
2. Cambios breaking en campos de `first_run` REQUIEREN RFC.

## Gate obligatorio

`release_phase10_first_run_gate_tests` DEBE validar este contrato.
