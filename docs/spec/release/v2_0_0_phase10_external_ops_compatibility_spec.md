# nC Phase10 External Ops Compatibility Spec v2.0.0

Estado: `frozen`

## Compatibilidad obligatoria

1. `publish_phase10_external_channels_v2_0_0.sh` DEBE mantener `--mode dry-run` como default seguro.
2. El reporte `docs/status/F10_external_publish_report.json` DEBE preservar `schema`, `version`, `status`, `operations`.
3. `smoke_phase10_clean_machine_matrix_v2_0_0.sh` DEBE seguir validando `ncup init`, `ncup doctor`, `ncc run` y chequeo low-level.
4. `ncc-release release-notes` DEBE regenerar `v2_0_0_release_notes.md` y `v2_0_0_release_announcement.md` sin intervención manual.
5. Cambios breaking en adapters de publicación, política de key/HSM o formato de reportes REQUIEREN RFC.
