# nC Phase10 External Operations Spec v2.0.0

Estado: `frozen`  
Fase: `phase10_external_ops`

## Objetivo

Cerrar de extremo a extremo la operación externa post-F10 para:

1. publicación multi-canal real (`winget`, `brew`, `deb`, `rpm`);
2. operación de claves productivas y modelo HSM/enterprise trust;
3. smoke clean-machine en runners limpios por OS;
4. release notes + announcement canónicos y reproducibles.

## Base activo

1. El compilador `ncc` DEBE seguir siendo la autoridad semántica del lenguaje.
2. `ncup` DEBE gobernar instalación/actualización/distribución operativa.
3. La publicación externa NO DEBE omitir verificación de integridad/firma.
4. Todo paso operativo DEBE producir reporte machine-readable.
5. El modo por defecto de publicación DEBE ser `dry-run`.

## 1) Publicación externa multi-canal

Script canónico:

- `tools/release/publish_phase10_external_channels_v2_0_0.sh`
- `tools/release/render_phase10_external_publish_env_v2_0_0.sh`
- `tools/release/run_phase10_external_publish_matrix_v2_0_0.sh`

Reglas:

1. DEBE validar existencia de artefactos + bundles de integridad por OS antes de publicar.
2. DEBE soportar `stable|beta|nightly`.
3. DEBE soportar `--mode dry-run|execute`.
4. `execute` REQUIERE adapter command explícito por canal (`NC_WINGET_SUBMIT_CMD`, `NC_BREW_TAP_PUBLISH_CMD`, `NC_DEB_PUBLISH_CMD`, `NC_RPM_PUBLISH_CMD`).
5. Si falta adapter o artefacto, DEBE registrar `deferred|skipped` con razón explícita.
6. Si un adapter contiene placeholders `REPLACE_ME_*`, DEBE fallar con diagnóstico explícito.
7. El wrapper de matriz DEBE mapear `linux->deb,rpm`, `windows->winget`, `macos->brew`, `all->winget,brew,deb,rpm`.

## 2) Claves productivas y HSM

Política operativa:

1. `ncup-init` mantiene trust root embebida de bootstrap.
2. La firma operativa de release NO DEBE depender de keys hardcodeadas en repositorio.
3. En producción, signing key DEBE residir en HSM/KMS o proceso equivalente administrado.
4. En enterprise, mirror + trust key interna DEBEN poder reemplazar trust root pública bajo política explícita.
5. Cualquier rotación de root/trust key REQUIERE runbook de rotación + ventana de compatibilidad.

## 3) Smoke clean-machine por OS

Script canónico:

- `tools/release/smoke_phase10_clean_machine_matrix_v2_0_0.sh`

Reglas:

1. DEBE ejecutar `ncup init` sobre home limpio.
2. DEBE ejecutar `ncup doctor`.
3. DEBE validar `ncc run` fuera del repo.
4. DEBE validar superficie low-level (`.nb/.nm/.ncp`) vía política `nclsp validate`.
5. DEBE generar reporte JSON.

## 4) Release notes y announcement

Generador canónico:

- `tools/ncc/nc/src/verification/release_notes.n`

Salidas canónicas:

- `docs/spec/release/v2_0_0_release_notes.md`
- `docs/spec/release/v2_0_0_release_announcement.md`

Reglas:

1. Las notas DEBEN reflejar estado real de reportes F10.
2. El anuncio NO DEBE declarar componentes sin evidencia en reportes.
3. Breaking notes DEBEN alinearse con contratos de compatibilidad activos.

## Verificación

Gate operativo:

- `ncc-tools-v43 dist-external-ops` ([build V43](../../NATIVE_TOOLS_V43_COMMANDS.md))
- `tools/release/qa_phase10_external_ops_v2_0_0.sh`

Reporte:

- `docs/status/F10_external_ops_report.json`

## Criterio de cierre

Este bloque queda cerrado cuando:

1. spec/compat/contract quedan en `frozen`;
2. `ncc-tools-v43 dist-external-ops` emite `status=ok`;
3. existe workflow CI dedicado para dry-run operativo externo;
4. release notes + announcement se regeneran de forma determinista.
