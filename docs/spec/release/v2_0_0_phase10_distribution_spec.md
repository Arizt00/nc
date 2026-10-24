# nC Phase10 Distribution Spec v2.0.0

Estado: `frozen`  
Fase: `phase10_installer_product`  
Alcance: distribución binaria multi-plataforma de `ncup-init`/`ncup` y bins oficiales.

Nota: la especialización por sistema operativo y paridad de layout lógico se formaliza adicionalmente en `v2_0_0_phase10_distribution_os_spec.md` (F10-P3).  
Nota adicional: la estructura de disco canónica de `ncup` se formaliza en `v2_0_0_phase10_disk_layout_spec.md` (F10-P4).

## 1. Artefactos obligatorios

La distribución DEBE producir bundles por target con bins:

- `ncc`
- `nfmt`
- `nlint`
- `nbuild`
- `npkg`
- `nide`
- `nclsp`
- `ncup`
- `ncup-init`

## 2. Matriz de targets canónica

Tier 1 de F10-P1:

- `x86_64-unknown-linux-gnu`
- `x86_64-pc-windows-msvc`
- `aarch64-apple-darwin`
- `wasm32-wasi`

La pipeline PUEDE ejecutar `--host-only` para smoke local, pero release formal DEBE cubrir la matriz aprobada.

## 3. Integridad

Cada corrida de distribución DEBE generar:

- `SHA256SUMS`
- `SHA256SUMS.sig`
- `SBOM.ncc.v1.json`
- `attestation.v1.json`

La firma/checksum se materializa mediante:

- `tools/release/sign_artifacts_v1_0_1.sh`
- `tools/release/verify_artifacts_v1_0_1.sh`

## 4. Instalación limpia

La distribución DEBE verificarse en entorno limpio:

- bootstrap con `ncup-init`;
- salud con `ncup doctor`;
- validación `nclsp` en `strict`.

Script de cierre:

- `tools/release/verify_phase10_clean_install_v2_0_0.sh`

## 5. Reportes

La distribución DEBE publicar:

- `docs/status/F10_P1_distribution_report.json`
- `docs/status/F10_P1_clean_install_report.json`

Con esto, F10-P1 mantiene trazabilidad técnica de build, integridad e instalación.
