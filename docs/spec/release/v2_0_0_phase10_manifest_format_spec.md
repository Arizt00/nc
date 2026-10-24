# nC Phase10 Manifest Format Spec v2.0.0

Estado: `frozen`  
Serie: `2.x`  
Fase: `phase10_p8_manifest_format`

## Objetivo

Cerrar el formato canónico de manifests de distribución/toolchain para consumo humano, `ncup` y tooling externo CI/CD.

F10-P8 define dos artefactos obligatorios:

1. `channel-manifest.toml`
2. `release-manifest.json`

## Base activo

### 1) Channel manifest

Ruta canónica:

- `manifests/channels/<channel>/channel-manifest.toml`

Canales obligatorios:

- `stable`
- `beta`
- `nightly`

Campos mínimos obligatorios:

- `schema`
- `version`
- `state`
- `channel`
- `min_ncup_version`
- `breaking_notes`
- `[compatibility]`
- `[components]`
- `[[entries]]` con:
  - `toolchain_id`
  - `target`
  - `artifact`
  - `url`
  - `sha256`
  - `components`

### 2) Release manifest

Ruta canónica:

- `manifests/release-manifest.json`

Campos mínimos obligatorios:

- `schema`
- `version`
- `state`
- `generated_at`
- `min_ncup_version`
- `breaking_notes`
- `channels`
- `components`
- `packages`
- `integrity_bundle`

`integrity_bundle` DEBE declarar:

- `checksums`
- `signature`
- `sbom`
- `attestation`

## Compatibilidad semántica declarada

F10-P8 exige que ambos manifests declaren explícitamente:

- `min_ncup_version`
- `breaking_notes`

Regla:

- `min_ncup_version` DEBE ser evaluable por `ncup` para rechazar consumidores obsoletos.
- `breaking_notes` DEBE existir siempre (lista vacía permitida).

## Integridad

Los hashes declarados en manifests DEBEN corresponder con artefactos reales y con el bundle de integridad firmado.

Regla mínima:

- `sha256` por entrada de paquete DEBE validar contra el artefacto real.

## Pipeline oficial

- `tools/release/build_phase10_ncup_distribution_v2_0_0.sh`
- `ncc-release-v43 release-manifest-format` ([build V43](../../NATIVE_TOOLS_V43_COMMANDS.md))
- `tools/release/qa_phase10_p8_v2_0_0.sh`
- `tools/release/qa_phase10_v2_0_0.sh`

## Gate oficial

- `release_phase10_manifest_format_gate_tests`

## Criterio de cierre

F10-P8 queda cerrado cuando:

1. spec/compat/contract están en `frozen`;
2. `build_phase10_ncup_distribution_v2_0_0.sh` emite manifests canónicos;
3. `ncc-release-v43 release-manifest-format` valida estructura + semántica + hashes;
4. reporte F10-P8 queda en `ok`;
5. gate y QA global quedan en verde.
