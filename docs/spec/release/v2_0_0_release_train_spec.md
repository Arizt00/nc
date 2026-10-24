# nC Release Train Spec v2.0.0

Estado: `frozen`  
Serie: `2.x`  
Fase: `phase10_release_train`

## Objetivo

Encadenar formalmente F10-P1..F10-P20 (incluyendo cierre IDE fullstack) al release train institucional, de forma que ninguna promoción de canal pueda omitir sus gates.

## Canales

- `stable`
- `beta`
- `nightly`

## Promoción

Flujo canónico:

1. `nightly -> beta`
2. `beta -> stable`

Script canónico de promoción 2.x:

- `tools/release/promote_release_train_v2_0_0.sh`

## Gates obligatorios en 2.x

- `release_phase1_language_base_gate_tests`
- `release_phase2_toolchain_model_gate_tests`
- `release_phase3_runtime_stdlib_gate_tests`
- `release_phase4_backend_sovereign_gate_tests`
- `release_phase5_toolchain_complete_gate_tests`
- `release_phase6_quality_extreme_gate_tests`
- `release_phase7_package_ecosystem_gate_tests`
- `release_phase8_dx_observability_gate_tests`
- `release_phase9_governance_release_gate_tests`
- `release_phase10_installer_gate_tests`
- `release_phase10_toolchain_model_gate_tests`
- `release_phase10_distribution_os_gate_tests`
- `release_phase10_disk_layout_gate_tests`
- `release_phase10_ncup_commands_gate_tests`
- `release_phase10_toolchain_resolution_gate_tests`
- `release_phase10_supply_chain_security_gate_tests`
- `release_phase10_manifest_format_gate_tests`
- `release_phase10_updates_gate_tests`
- `release_phase10_jetbrains_gate_tests`
- `release_phase10_vscode_gate_tests`
- `release_phase10_ghidra_gate_tests`
- `release_phase10_file_association_global_exec_gate_tests`
- `release_phase10_first_run_gate_tests`
- `release_phase10_cicd_compat_gate_tests`
- `release_phase10_telemetry_gate_tests`
- `release_phase10_installer_matrix_gate_tests`
- `release_phase10_support_policy_gate_tests`
- `release_phase10_documentation_gate_tests`
- `release_phase10_final_closure_gate_tests`
- `release_phase10_release_train_gate_tests`
- `release_primitive_material_gate_tests`
- `release_f23_compatibility_gate_tests`
- `release_f24_release_engineering_gate_tests`

## Regla de promoción

NO DEBE promoverse ningún canal si cualquiera de los gates obligatorios falla.

## Estado y persistencia

- estado de canales: `var/release/release_train_state_v2_0_0.json`
- planes de promoción: `var/release/train/promotions/*_v2_0_0.json`


## Operación externa obligatoria

Antes de promoción efectiva de `stable`, la operación externa DEBE validar:

- `tools/release/qa_phase10_external_ops_v2_0_0.sh`
- `docs/status/F10_external_ops_report.json` en `ok`
- workflow operativo: `.github/workflows/release-phase10-external-ops.yml`
