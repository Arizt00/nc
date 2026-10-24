# nC Phase10 Updates Spec v2.0.0

Estado: `frozen`  
Fase: `phase10_p9_updates`

## Objetivo

Cerrar el comportamiento de actualizaciones de `ncup` con garantías de producto maduro:

- `ncup update` DEBE actualizar solo la toolchain activa efectiva.
- la actualización DEBE ser transaccional con validación previa a commit.
- si la instalación parcial falla, DEBE aplicar rollback automático.
- DEBE mantener historial de versiones instaladas para revertir.

## Resolución de toolchain activa

`ncup update` DEBE resolver la toolchain activa por orden determinista:

1. override local (`nc-toolchain.toml`);
2. variable de entorno `NCUP_TOOLCHAIN`;
3. default global en estado `ncup`.

La actualización solo puede afectar la toolchain resuelta en ese orden.

## Transacción de update

`ncup update` DEBE ejecutar una transacción con estos pasos:

1. snapshot de estado previo y snapshot del directorio de la toolchain activa;
2. reinstalación/refresh de bins + manifest + integridad;
3. validación obligatoria de integridad antes de commit;
4. commit de estado si y solo si todas las validaciones pasan.

## Rollback automático

Si falla cualquier paso post-snapshot:

- DEBE restaurar estado previo;
- DEBE restaurar el directorio de toolchain desde snapshot;
- DEBE re-sincronizar proxies por defecto cuando aplique;
- DEBE registrar evento `rolled_back` en historial.

## Historial para revertir

`ncup` DEBE persistir `manifests/update-history.v1.json` con entradas inmutables de actualización.

Cada entrada DEBE incluir como mínimo:

- `id` de transacción;
- `toolchain_id`;
- `resolved_from` y `resolved_detail`;
- `before_revision` y `after_revision`;
- `status` (`ok` o `rolled_back`);
- `rollback_applied`;
- `snapshot_dir`.

## Gate oficial

- `release_phase10_updates_gate_tests`

## Pipeline de verificación

- `ncc-release-v43 release-updates` ([build V43](../../NATIVE_TOOLS_V43_COMMANDS.md))
- `tools/release/qa_phase10_p9_v2_0_0.sh`

## Criterio de cierre

F10-P9 queda cerrado cuando:

1. contrato/spec/compat en `frozen`;
2. `ncup update` no toca toolchains no activas;
3. fallo inyectado produce rollback automático verificable;
4. historial de updates se emite en estado `ok`;
5. gate dedicado pasa en CI/local y está en QA global de phase10.
