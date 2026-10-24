# nC Phase10 Distribution by OS Spec v2.0.0

Estado: `frozen`  
Fase: `phase10_p3_distribution_os`

## Objetivo

Cerrar distribución multi-OS para serie `2.x` con paridad de layout lógico de toolchain.

## Base activo

La distribución por OS DEBE producir estas superficies:

- Windows: `MSI` + `winget manifest` + `portable zip` firmado.
- Linux: `install script (.sh)` + `tarball` + `deb/rpm` (incremental, pero contractual).
- macOS: `pkg` + `brew tap`.

## Regla de paridad lógica

Todos los formatos DEBEN transportar la misma estructura lógica:

- `toolchain/bin/{ncc,nfmt,nlint,nbuild,npkg,nide,nclsp,nwal,ncup,ncup-init}`
- `toolchain/manifest.v1.json`
- `toolchain/layout.v1.json`

La verificación de paridad DEBE ejecutarse con:

- `tools/ncc/nc/src/verification/release_layout.n`

Se ejecuta como `ncc-release release-layout --staging DIR --report JSON`.
El binario nativo se construye desde `tools/ncc/nc` mediante Stage0; no requiere
Python para esta verificacion. Los scripts de empaquetado admiten
`NCC_RELEASE_VERIFY` para seleccionar su ruta. Instalacion y limites:
`docs/PYTHON_RETIREMENT_V38.md`.

## Pipeline canónico

Script principal:

- `tools/release/build_phase10_distribution_os_v2_0_0.sh`

QA de fase:

- `tools/release/qa_phase10_p3_v2_0_0.sh`

## Política de limpieza RPM en QA

Para entorno Linux, la build RPM DEBE ejecutarse con base de datos RPM aislada de host (`_dbpath` bajo staging temporal del pipeline) y changelog válido RFC-compatible.

La fase QA DEBE fallar si detecta firmas de warning/errores heredados de host RPM, incluyendo:

- `rpmdb.sqlite`
- `cannot open Packages database`
- `bogus date in %changelog`

Integridad:

- `SHA256SUMS`
- `SHA256SUMS.sig`
- `SBOM.ncc.v1.json`
- `attestation.v1.json`

## Política native-runner

`MSI` y `pkg` son artefactos native-runner.

- Si el runner no provee toolchain de empaquetado nativa, la pipeline DEBE emitir artefacto `deferred` con razón explícita.
- En release CI por matriz de OS, esos artefactos DEBEN materializarse sin `deferred`.

## Criterio de cierre

F10-P3 queda cerrado cuando:

1. contrato y spec están en `frozen`;
2. la pipeline produce artefactos por OS con integridad;
3. la paridad lógica es validada automáticamente;
4. el gate `release_phase10_distribution_os_gate_tests` está en verde.
