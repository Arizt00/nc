# nC Phase10 Installer Matrix Compatibility Spec v2.0.0

Estado: `frozen`  
Serie: `2.x`

## Compatibilidad operativa

1. La matriz DEBE validar flujos reales de `ncup-init` y `ncup`.
2. La matriz NO DEBE introducir semántica paralela al compilador.
3. Las pruebas de fallo DEBEN ser controladas y reproducibles.

## Compatibilidad de escenarios

- `fresh install`, `upgrade`, `downgrade`, `rollback`: obligatorios.
- `corrupt download`, `invalid signature`, `insufficient space`: obligatorios.
- `PATH empty`, `PATH existing`: obligatorios.
- `multi-toolchain` + `project override`: obligatorios.

## Compatibilidad de runners

La ejecución institucional DEBE declararse en runner matrix limpio:

- Linux
- Windows
- macOS

## Gate obligatorio

`release_phase10_installer_matrix_gate_tests` DEBE permanecer en verde para 2.x.
