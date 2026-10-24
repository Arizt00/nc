# nC Phase10 Supply Chain Security Compatibility Spec v2.0.0

Estado: `frozen`  
Serie: `2.x`

## Compatibilidad activa

- La verificación de integridad en `ncup` DEBE mantenerse obligatoria en serie `2.x`.
- `ncup verify` DEBE mantenerse estable como superficie canónica de validación de cadena de suministro.
- `keys/root-trust-key.v1.json` DEBE preservarse como contrato de bootstrap de confianza.
- El modo enterprise (`--enterprise-mirror`, `--enterprise-trust-key`) DEBE mantenerse compatible.
- La estructura `integrity/*` de toolchain y distribución NO DEBE romperse sin RFC.

## Reglas

1. Cambios breaking de key model, firma o estructura de manifests requieren RFC.
2. Nuevos metadatos en attestation/manifests PUEDEN añadirse de forma aditiva.
3. Nuevos algoritmos criptográficos PUEDEN incorporarse, pero los actuales NO DEBEN eliminarse sin transición formal.
4. `ncup doctor` y `ncup verify` NO DEBEN depender del IDE/editor.
5. El modo enterprise PUEDE ampliar políticas de trust, pero NO DEBE degradar validación obligatoria.
