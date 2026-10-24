# nC Phase10 Installer Product Spec v2.0.0

Estado: `frozen`
Fase: `phase10_installer_product`

## Objetivo

Cerrar el producto instalable de nC con autoridad operativa formal para:

- bootstrap inicial con `ncup-init`;
- gestión persistente con `ncup`;
- bins oficiales de toolchain;
- distribución por OS con paridad de layout lógico de toolchain;
- actualizaciones transaccionales con rollback e historial;
- política de migración controlada de extensiones low-level;
- frontera LSP general/especializada (`nclsp` + `nc-sys-lsp`);
- experiencia first-run cerrada (`wizard + doctor + hello.n + guía`);
- compatibilidad CI/CD oficial (`GitHub Action + GitLab template + cache por hash + offline`).
- telemetría de instalación `opt-in-only` con privacidad explícita.
- matriz de pruebas del instalador (`fresh/upgrade/downgrade/rollback/corrupción/firma/PATH/override/OS matrix`).
- política de soporte por canal (`stable` productivo, `beta` preview con migración, `nightly` experimental sin SLA) + ventana LTS enterprise.
- documentación obligatoria de instalación/toolchains/IDEs/seguridad/CI (`INSTALL.md`, `TOOLCHAINS.md`, `IDE_JETBRAINS.md`, `IDE_VSCODE.md`, `IDE_GHIDRA.md`, `SECURITY_SUPPLY_CHAIN.md`, `CI_SETUP.md`).
- cierre final integral de Fase 10 (F10-P19) con certificación machine-readable.

## Política canónica de migración de extensiones low-level

La migración `.n -> .nb/.nm/.ncp` DEBE operar por etapas:

1. `warning-first`
- legado `.n` en familias low-level produce warning.
- no rompe compilación por extensión.

2. `error-by-profile`
- `novice`: warning.
- `standard`: error.
- `expert`: error.

3. `strict`
- todos los perfiles producen error para legado `.n` low-level.

Regla: la migración aplica SOLO al legado `.n` low-level. Cualquier otro mismatch de extensión permanece error directo.

## Bins oficiales F10-P1

- `ncc`
- `nfmt`
- `nlint`
- `nbuild`
- `npkg`
- `nide`
- `nclsp`
- `nwal`
- `ncup`
- `ncup-init`

Componente opcional low-level:

- `nc-sys-lsp` (incremental, especializado, no duplicador de `nclsp`).

## Gate formal obligatorio

El cierre F10-P1 REQUIERE:

- módulo de tests `release_phase10_installer_gate_tests`;
- módulo de tests `release_phase10_toolchain_model_gate_tests`;
- módulo de tests `release_phase10_distribution_os_gate_tests`;
- módulo de tests `release_phase10_disk_layout_gate_tests`;
- módulo de tests `release_phase10_updates_gate_tests`;
- módulo de tests `release_phase10_jetbrains_gate_tests`;
- módulo de tests `release_phase10_vscode_gate_tests`;
- módulo de tests `release_phase10_ghidra_gate_tests`;
- módulo de tests `release_phase10_file_association_global_exec_gate_tests`;
- módulo de tests `release_phase10_first_run_gate_tests`;
- módulo de tests `release_phase10_cicd_compat_gate_tests`;
- módulo de tests `release_phase10_telemetry_gate_tests`;
- módulo de tests `release_phase10_installer_matrix_gate_tests`;
- módulo de tests `release_phase10_support_policy_gate_tests`;
- módulo de tests `release_phase10_documentation_gate_tests`;
- módulo de tests `release_phase10_final_closure_gate_tests`;
- módulo de tests `release_phase10_release_train_gate_tests`;
- script de release `tools/release/qa_phase10_v2_0_0.sh`;
- script de release `tools/release/qa_phase10_p2_v2_0_0.sh`;
- script de release `tools/release/qa_phase10_p3_v2_0_0.sh`;
- script de migración wave1+wave2+wave3+wave4 `tools/release/migrate_low_level_extensions_v2_0_0.sh`;
- script de distribución `tools/release/build_phase10_ncup_distribution_v2_0_0.sh`;
- script de distribución OS `tools/release/build_phase10_distribution_os_v2_0_0.sh`;
- verificador de paridad `tools/ncc/nc/src/verification/release_layout.n`;
- instalador Linux oficial `tools/release/install_ncup_linux_v2_0_0.sh`;
- verificador de layout canónico `tools/ncc/nc/src/verification/ncup_disk.n`;
- script de gate layout canónico `tools/release/qa_phase10_p4_v2_0_0.sh`;
- comando nativo de verificación de updates `ncc-release-v43 release-updates`; ([build V43](../../NATIVE_TOOLS_V43_COMMANDS.md))
- script de gate updates `tools/release/qa_phase10_p9_v2_0_0.sh`;
- comando nativo de verificación JetBrains `ncc-ide-v43 ide43-jetbrains`; ([build V43](../../NATIVE_TOOLS_V43_COMMANDS.md))
- script de gate JetBrains `tools/release/qa_phase10_p10_v2_0_0.sh`;
- comando nativo de verificación VSCode `ncc-ide-v43 ide43-vscode`; ([build V43](../../NATIVE_TOOLS_V43_COMMANDS.md))
- script de gate VSCode `tools/release/qa_phase10_p11_v2_0_0.sh`;
- comando nativo de verificación Ghidra `ncc-ide-v43 ide43-ghidra`; ([build V43](../../NATIVE_TOOLS_V43_COMMANDS.md))
- script de gate Ghidra `tools/release/qa_phase10_p20_v2_0_0.sh`;
- script de distribución IDE fullstack `tools/release/build_phase10_ide_distribution_v2_0_0.sh`;
- comando nativo de verificación IDE fullstack `ncc-ide-v43 ide43-fullstack`; ([build V43](../../NATIVE_TOOLS_V43_COMMANDS.md))
- script de gate IDE fullstack `tools/release/qa_phase10_ide_fullstack_v2_0_0.sh`;
- script de publicación marketplaces IDE `tools/release/publish_phase10_ide_marketplaces_v2_0_0.sh`;
- script de plantilla de entorno marketplaces IDE `tools/release/render_phase10_ide_marketplace_env_v2_0_0.sh`;
- runbook marketplaces IDE `docs/spec/release/v2_0_0_phase10_ide_marketplace_runbook.md`;
- workflow CI IDE fullstack `.github/workflows/phase10-ide-fullstack.yml`;
- comando nativo de verificación asociación/ejecución global `ncc-ide-v43 ide43-association`; ([build V43](../../NATIVE_TOOLS_V43_COMMANDS.md))
- script de gate asociación/ejecución global `tools/release/qa_phase10_p12_v2_0_0.sh`;
- script de verificación first-run `tools/ncc/nc/src/verification/ncup_first_run.n`;
- script de gate first-run `tools/release/qa_phase10_p13_v2_0_0.sh`;
- script CI canónico `tools/ci/setup_ncup_ci_v2_0_0.sh`;
- wrappers CI `tools/ci/github/setup_ncup_ci_v2_0_0.sh` y `tools/ci/gitlab/setup_ncup_ci_v2_0_0.sh`;
- acción oficial GitHub `.github/actions/setup-nc/action.yml`;
- template oficial GitLab `.gitlab-ci.ncup.yml`;
- comando nativo de verificación CI/CD `ncc-release-v43 release-cicd-compat`; ([build V43](../../NATIVE_TOOLS_V43_COMMANDS.md))
- script de gate CI/CD `tools/release/qa_phase10_p14_v2_0_0.sh`;
- comando nativo de verificación de telemetría `ncc-tools-v43 dist-telemetry`; ([build V43](../../NATIVE_TOOLS_V43_COMMANDS.md))
- script de gate de telemetría `tools/release/qa_phase10_p15_v2_0_0.sh`;
- comando nativo de verificación de matriz del instalador `ncc-release-v43 release-installer-matrix`; ([build V43](../../NATIVE_TOOLS_V43_COMMANDS.md))
- script de gate de matriz del instalador `tools/release/qa_phase10_p16_v2_0_0.sh`;
- script de verificación de política de soporte `tools/ncc/nc/src/verification/support_policy.n`;
- script de gate de política de soporte `tools/release/qa_phase10_p17_v2_0_0.sh`;
- script de verificación de documentación obligatoria `tools/ncc/nc/src/verification/documentation_check.n`;
- script de gate de documentación obligatoria `tools/release/qa_phase10_p18_v2_0_0.sh`;
- script de verificación de cierre final `tools/ncc/nc/src/verification/final_closure.n`;
- script de gate de cierre final `tools/release/qa_phase10_p19_v2_0_0.sh`;
- workflow de runners limpios multiplataforma `.github/workflows/phase10-installer-matrix.yml`;
- script de instalación limpia `tools/release/verify_phase10_clean_install_v2_0_0.sh`;
- script de snapshot toolchain F10-P2 `tools/release/snapshot_phase10_toolchain_model_v2_0_0.sh`;
- contrato machine-readable `v2_0_0_phase10_installer_product_contract.json`.

## Criterio de madurez

F10-P1 se considera operativo solo si:

- `nclsp validate` soporta perfil editorial + etapa de migración;
- `ncup model` refleja el producto canónico;
- `nc-sys-lsp` supera `info`, `doctor`, `analyze`, `serve` incremental y `lsp` JSON-RPC;
- distribución genera paquetes + checksum/firma/SBOM;
- distribución por OS mantiene layout lógico único `nc.toolchain.layout.v1`;
- instalación limpia valida bootstrap y bins sin dependencia del repo;
- first-run ejecuta doctor + demo (`ncc run hello.n`) y entrega guía post-instalación;
- setup CI soporta `channel+target+components` con cache por `sha256(manifest)` y modo offline;
- el gate de fase pasa en CI/local sin hacks.
