# nC Primitive[@] Release Evidence Spec v2.0.0

Estado: `frozen`
Fase: `primitive_release_evidence`

## Objetivo

Este gate DEBE cerrar evidencia material de `primitive[@]` antes de avanzar a otro modo.

El cierre REQUIERE:

1. Build booteable real (`.sov`) para `x86_64-unknown-none`.
2. Build freestanding ELF real (`.elf`).
3. Smoke QEMU serial con evidencia textual `nC primitive alive`.
4. Paquete de distribución reproducible con checksums SHA-256.
5. Reporte machine-readable en `docs/status/PRIMITIVE_release_evidence_report_v2_0_0.json`.

## Base activo

1. `primitive[@]` NO DEBE depender de runtime alto nivel.
2. El smoke QEMU DEBE validar imagen de 512 bytes y firma `0x55AA`.
3. El gate DEBE verificar opcodes mínimos materiales (`ba,ee,ec,a8,f4`).
4. La evidencia DEBE incluir rutas de artefactos, firma, tamaño y magic ELF.
5. El packaging DEBE incluir manifest + `SHA256SUMS`.

## Artefactos normativos

- `targets/x86_64-unknown-none.nctarget`
- `link/x86_64_baremetal.ld`
- `examples/primitive/boot_serial_hello.ncp`
- `tools/scripts/run_primitive_qemu_smoke.sh`
- `ncc-certification-v43 primitive-release-evidence` ([build V43](../../NATIVE_TOOLS_V43_COMMANDS.md))
- `tools/release/qa_primitive_release_evidence_v2_0_0.sh`

## Salidas obligatorias

- `var/primitive/qemu-smoke/boot_serial_hello.sov`
- `var/primitive/qemu-smoke/boot_serial_hello.elf`
- `var/primitive/qemu-smoke/qemu_serial.log`
- `var/primitive/qemu-smoke/material_evidence.txt`
- `var/releases/v2_0_0/primitive_material_dist/packages/primitive-material-x86_64-unknown-none-v2.0.0.tar.gz`
- `var/releases/v2_0_0/primitive_material_dist/integrity/SHA256SUMS`
- `var/releases/v2_0_0/primitive_material_dist/manifests/primitive-release-manifest.json`

## Gate oficial

- `release_primitive_material_gate_tests`

## Criterio de cierre

El gate queda cerrado solo si:

1. `qa_primitive_release_evidence_v2_0_0.sh` termina en `OK`.
2. `ncc-certification-v43 primitive-release-evidence` emite status `ok`.
3. El log de QEMU contiene `nC primitive alive`.
4. El paquete y checksums existen y son no vacíos.
