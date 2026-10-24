# nC Phase10 Installer Compatibility Spec v2.0.0

Estado: `frozen`
Serie: `2.x`

## Compatibilidad estructural

1. El compilador (`ncc`) permanece autoridad semántica.
2. La política de extensión pertenece a ecosistema/editorial, no a semántica core.
3. `nclsp` y `nc-sys-lsp` NO DEBEN emitir leyes semánticas en conflicto con `ncc`.

## Compatibilidad de migración

- Etapa por defecto: `warning-first`.
- Escalada por perfil: disponible mediante `error-by-profile`.
- Endurecimiento absoluto: `strict`.

Sin contradicción formal posterior, este orden DEBE mantenerse para evitar ruptura abrupta del corpus legacy `.n`.

## Compatibilidad de bins

La instalación mínima DEBE seguir siendo estable con:

- `ncc`
- `ncup`
- `ncup-init`

La instalación `default` y `complete` DEBEN conservar los bins de F10-P1 sin renombrados breaking.

## Compatibilidad de distribución por OS

- Windows, Linux y macOS DEBEN mantener layout lógico único `nc.toolchain.layout.v1`.
- Cambios en rutas lógicas de toolchain requieren RFC.
- Formatos native-runner (`msi`/`pkg`) pueden declararse `deferred` fuera de runner nativo, pero NO en release matrix final.

## Compatibilidad LSP

- `nclsp` sigue siendo superficie LSP general.
- `nc-sys-lsp` es componente opcional especializado low-level con adapter JSON-RPC.
- Cualquier futura expansión de `nc-sys-lsp` DEBE mantenerse complementaria, no duplicada.

## Compatibilidad CI/CD

1. La integración oficial DEBE seguir expuesta por action GitHub y template GitLab.
2. El setup CI DEBE mantener cache por hash de manifest.
3. El setup CI DEBE mantener modo offline con bundle predescargado.

## Gate obligatorio

`release_phase10_installer_gate_tests`, `release_phase10_toolchain_model_gate_tests`, `release_phase10_distribution_os_gate_tests`, `release_phase10_disk_layout_gate_tests`, `release_phase10_ncup_commands_gate_tests`, `release_phase10_toolchain_resolution_gate_tests`, `release_phase10_supply_chain_security_gate_tests`, `release_phase10_manifest_format_gate_tests`, `release_phase10_updates_gate_tests`, `release_phase10_jetbrains_gate_tests`, `release_phase10_vscode_gate_tests`, `release_phase10_file_association_global_exec_gate_tests`, `release_phase10_first_run_gate_tests`, `release_phase10_cicd_compat_gate_tests`, `release_phase10_telemetry_gate_tests`, `release_phase10_installer_matrix_gate_tests`, `release_phase10_support_policy_gate_tests`, `release_phase10_documentation_gate_tests`, `release_phase10_final_closure_gate_tests` y `release_phase10_release_train_gate_tests` DEBEN validar este contrato y su artefacto machine-readable.
