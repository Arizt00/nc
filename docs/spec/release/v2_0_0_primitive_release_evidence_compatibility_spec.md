# nC Primitive[@] Release Evidence Compatibility Spec v2.0.0

Estado: `frozen`

## Compatibilidad

1. El formato de `material_evidence.txt` DEBE mantener claves estables:
   - `primitive-image-size`
   - `primitive-boot-signature`
   - `primitive-opcodes`
   - `primitive-elf-magic`
   - `qemu-log`
2. El manifest `primitive-release-manifest.json` DEBE conservar esquema `nc.primitive.release.manifest.v1`.
3. `SHA256SUMS` DEBE mantenerse en formato `<sha256><2 espacios><ruta_relativa>`.
4. Cambios breaking REQUIEREN RFC y actualización de este spec.

## No-breaking policy

Hasta contradicción formal posterior:

- NO se permiten cambios breaking en rutas de salida, nombres de artefactos o claves de evidencia.
