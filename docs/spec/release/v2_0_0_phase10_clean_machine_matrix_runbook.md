# nC v2.0.0 Clean-Machine Matrix Runbook

Estado: `frozen`

## Objetivo

Validar instalación y ejecución de nC en runners limpios por OS, fuera del repo fuente.

## Script canónico

- `tools/release/smoke_phase10_clean_machine_matrix_v2_0_0.sh`

## Casos mínimos

1. `ncup init` en home limpio.
2. `ncup doctor`.
3. `ncup which ncc --json`.
4. `ncc run hello.n` fuera del repo.
5. `nclsp validate` para superficie low-level canónica.

## Ejecución

```bash
bash tools/release/smoke_phase10_clean_machine_matrix_v2_0_0.sh
```

Salida:

- `docs/status/F10_external_clean_machine_report.json`

## Criterio

- `status=ok` en cada runner objetivo (`windows`, `linux`, `macos`).
- Cualquier fallo bloquea publicación de canal estable.
