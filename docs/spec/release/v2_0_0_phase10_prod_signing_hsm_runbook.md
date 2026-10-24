# nC v2.0.0 Production Signing & HSM Runbook

Estado: `frozen`

## Propósito

Definir el modelo operativo de claves productivas para signing de manifests y bins.

## Reglas obligatorias

1. La root trust key de bootstrap de `ncup-init` DEBE ser pública/verificable y versionada.
2. La signing key de release NO DEBE almacenarse en texto plano en repositorios, scripts ni artefactos intermedios.
3. Producción DEBE usar HSM/KMS (o servicio equivalente con acceso auditado y rotación).
4. Rotación de claves DEBE ser compatible con ventana de transición y doble firma cuando aplique.

## Modelo recomendado

- Root trust key id: `ncup-root-v2-2026` (bootstrap verification).
- Release signing: alias KMS/HSM por canal (`ncup-release-stable`, `ncup-release-beta`, `ncup-release-nightly`).
- Enterprise trust override: `--enterprise-mirror` + `--enterprise-trust-key`.

## Procedimiento operativo

1. Inicializar sesión de firma con identidad de release y MFA.
2. Firmar `SHA256SUMS` y manifests mediante endpoint HSM/KMS.
3. Publicar artefactos solo con firma válida.
4. Ejecutar `ncup verify` sobre toolchain resultante.
5. Registrar evidencias en auditoría de release.

## Rotación

1. Crear nueva key versionada.
2. Habilitar doble firma temporal.
3. Actualizar manifiestos de trust.
4. Retirar key anterior tras ventana de compatibilidad.

## Prohibiciones

- Prohibido usar keys locales sin custodia para publicación estable.
- Prohibido saltar verificación previa de firma/checksum.
