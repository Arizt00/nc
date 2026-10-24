# nC Phase10 Installer Matrix Spec v2.0.0

Estado: `frozen`  
Fase: `phase10_p16_installer_matrix`

## Objetivo

Cerrar F10-P16 con una matriz de validación de instalador completa, determinista y reproducible para `ncup-init`/`ncup`:

1. fresh install;
2. upgrade;
3. downgrade;
4. rollback transaccional en fallo;
5. detección de corrupción de descarga;
6. detección de firma inválida;
7. manejo de condición equivalente a falta de espacio;
8. comportamiento con PATH vacío y PATH preexistente;
9. multi-toolchain + override por proyecto;
10. contrato de runners limpios Windows/Linux/macOS.

## Reglas canónicas

1. La matriz DEBE ejecutarse de forma no destructiva sobre `NCUP_HOME` temporal.
2. Toda operación de instalación DEBE mantener verificabilidad por integridad (`sha256` + firma).
3. El flujo `update` DEBE ser transaccional y DEBE aplicar rollback cuando falle.
4. La simulación de espacio insuficiente DEBE producir diagnóstico claro y determinista.
5. El flujo de PATH DEBE funcionar tanto con PATH vacío como con PATH preexistente.
6. El override por proyecto (`nc-toolchain.toml`) DEBE prevalecer sobre default global.
7. La cobertura de OS DEBE quedar declarada en runner matrix limpio para:
- `ubuntu-latest`
- `windows-latest`
- `macos-latest`

## Gate oficial

- `release_phase10_installer_matrix_gate_tests`

## Pipeline de verificación

- `ncc-release-v43 release-installer-matrix` ([build V43](../../NATIVE_TOOLS_V43_COMMANDS.md))
- `tools/release/qa_phase10_p16_v2_0_0.sh`
- `.github/workflows/phase10-installer-matrix.yml`

## Criterio de cierre

F10-P16 queda cerrado cuando:

1. spec/compat/contract P16 están en `frozen`;
2. reporte `docs/status/F10_P16_installer_matrix_report.json` queda en `ok`;
3. la matriz cubre los escenarios obligatorios del contrato sin hacks;
4. el gate P16 queda integrado en QA global F10 y release train 2.x.
