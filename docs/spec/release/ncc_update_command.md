# ncc update command

Estado: `draft`

`ncc update` es la fachada de usuario final para actualizar binarios de la
toolchain nC con una fase previa obligatoria de planificacion.

## Contrato

- por defecto actualiza solo la toolchain efectiva activa, igual que `ncup update`;
- `--all-targets` actualiza todas las toolchains instaladas que pertenecen a la
  matriz baseica de targets;
- antes de modificar binarios imprime modo, alcance, toolchains y peso estimado;
- sin `--yes`, pide autorizacion interactiva;
- `--dry-run` solo imprime el plan;
- `--manifest <path>` permite declarar el peso desde un manifest JSON de release;
- `--download <url|file|dir>` descarga o toma un artefacto local, lo guarda bajo
  `downloads/`, extrae `.zip`, `.tar` o `.tar.gz` cuando aplica, y lo usa como
  bundle de binarios;
- `--bundle <dir>` instala binarios reales desde un bundle predescargado con
  forma `<bundle>/<toolchain-id>/bin` o `<bundle>/bin`;
- cada toolchain se actualiza con snapshot, validacion de integridad e historial
  de rollback en `manifests/update-history.v1.json`.

## Ejemplos

```bash
ncc update --dry-run
ncc update --yes
ncc update --all-targets --yes
ncc update --download https://example.invalid/nc-toolchain.tar.gz --yes
ncc update --target x86_64-pc-windows-msvc --bundle ./dist --yes
ncc update --manifest ./manifests/release-manifest.json --dry-run --json
```

## Relacion con ncup

La spec congelada de F10-P9 mantiene `ncup update` como operacion activa-only.
`ncc update` no cambia ese contrato; lo envuelve para uso de alto nivel y anade
planificacion, peso, autorizacion y seleccion multi-toolchain instalada.
