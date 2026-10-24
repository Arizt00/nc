# nC v2.0.0 Release Notes

Generated: 2026-05-04T19:42:39Z

## Estado del release

- Serie: `2.x`
- Estado: `frozen`
- Alcance: cierre F10-P1..F10-P19 + operación externa

## Matriz F10

| Punto | Reporte | Estado |
|---|---|---|
| P1 | `F10_P1_distribution_report.json` | `ok` |
| P2 | `F10_P2_toolchain_model_report.json` | `ok` |
| P3 | `F10_P3_distribution_os_report.json` | `ok` |
| P4 | `F10_P4_disk_layout_report.json` | `ok` |
| P5 | `F10_P5_ncup_commands_report.json` | `ok` |
| P6 | `F10_P6_toolchain_resolution_report.json` | `ok` |
| P7 | `F10_P7_supply_chain_security_report.json` | `ok` |
| P8 | `F10_P8_manifest_format_report.json` | `ok` |
| P9 | `F10_P9_updates_report.json` | `ok` |
| P10 | `F10_P10_jetbrains_integration_report.json` | `ok` |
| P11 | `F10_P11_vscode_integration_report.json` | `ok` |
| P12 | `F10_P12_file_association_global_exec_report.json` | `ok` |
| P13 | `F10_P13_first_run_report.json` | `ok` |
| P14 | `F10_P14_cicd_compat_report.json` | `ok` |
| P15 | `F10_P15_telemetry_report.json` | `ok` |
| P16 | `F10_P16_installer_matrix_report.json` | `ok` |
| P17 | `F10_P17_support_policy_report.json` | `ok` |
| P18 | `F10_P18_documentation_report.json` | `ok` |
| P19 | `F10_P19_final_closure_report.json` | `ok` |

## Operación externa

- External publish report: `partial`
- Publicación externa canónica: `tools/release/publish_phase10_external_channels_v2_0_0.sh`
- Smoke clean-machine: `tools/release/smoke_phase10_clean_machine_matrix_v2_0_0.sh`

## Seguridad

- Verificación SHA-256 y firma obligatoria antes de instalación.
- Modelo enterprise con mirror + trust key interno.

## Notas de compatibilidad

- No se permiten breaking changes sin RFC formal.
- La política de migración de extensiones low-level continúa warning-first -> error-by-profile -> strict.
