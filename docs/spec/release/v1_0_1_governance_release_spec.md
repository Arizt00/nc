# nC Governance + Release Cycle Spec v1.0.1

Estado: `frozen`  
Serie: `1.x`  
Fase: `phase9_governance_release`

## Objetivo

Cerrar el modelo institucional de releases para `1.x` con reglas explícitas y auditables sobre:

- release train `stable`, `beta`, `nightly`;
- política de retrocompatibilidad y cambios breaking;
- proceso RFC obligatorio;
- deprecaciones con ventanas mínimas;
- política LTS y soporte empresarial.

## Release train canónico

- `stable`:
  - cadencia objetivo: 28 días;
  - acepta cambios `PATCH` y `MINOR` compatibles;
  - sin cambios breaking.
- `beta`:
  - cadencia objetivo: 7 días;
  - validación previa a `stable`;
  - sin cambios breaking.
- `nightly`:
  - cadencia objetivo: 1 día;
  - integración temprana y validación continua;
  - sin cambios breaking en `1.x`.

Promoción oficial:

1. `nightly -> beta`
2. `beta -> stable`

Solo se promueve si los gates obligatorios están en verde.

## Gobernanza institucional

- RFC:
  - proceso: `docs/spec/release/rfc/README.md`;
  - plantilla: `docs/spec/release/rfc/RFC_TEMPLATE.md`;
  - obligatorio para breaking/deprecaciones con remoción/cambios ABI y runtime contract.
- deprecaciones:
  - aviso en release notes + changelog + warning de compilador;
  - mínimo 2 releases `stable` antes de remoción.
- LTS:
  - canal base: `stable`;
  - soporte objetivo: 24 meses;
  - backports: seguridad, bugs críticos, data-loss.
- soporte empresarial:
  - tracks soportados: `stable` y `lts`;
  - SLA seguridad: 24h;
  - objetivo hotfix crítico: 72h.

## Política de compatibilidad y predictibilidad

Para serie `1.x`:

- changes breaking: solo en `MAJOR`;
- sin breaking sin RFC aprobado;
- cada cambio debe explicitar impacto y ventana;
- usuarios deben saber exactamente qué rompe, cuándo y por qué.

## DoD de Fase 9

Fase 9 se considera cerrada solo si:

1. spec + compat + contract están en `frozen`;
2. gate `release_phase9_governance_release_gate_tests` está en verde;
3. release train es consumible por CLI/modelo machine-readable;
4. política de retrocompatibilidad incluye RFC/deprecaciones/LTS/enterprise sin ambigüedad;
5. la documentación permite anticipar con precisión qué rompe, cuándo y por qué.

## Endurecimiento operativo obligatorio (post-cierre)

Para nivel de lenguaje maduro, Fase 9 incluye endurecimiento operativo ejecutable:

1. Automatización CI/CD de promoción `nightly -> beta -> stable`:
   - workflow: `.github/workflows/release-train-promotion.yml`
   - script: `tools/release/promote_release_train_v1_0_1.sh`
2. Warnings reales de deprecación en compilación:
   - códigos: `NCC-DEPRECATION-0001`, `NCC-DEPRECATION-0002`
   - ventana mínima antes de remoción: 2 releases `stable`
3. Integridad de artefactos:
   - `SHA256SUMS` + firma HMAC + SBOM (`ncc.sbom.v1`) + attestation (`ncc.release.integrity.attestation.v1`)
   - scripts: `sign_artifacts_v1_0_1.sh`, `verify_artifacts_v1_0_1.sh`, `generate_sbom_v1_0_1.sh`
   - workflow: `.github/workflows/release-artifact-integrity.yml`
4. Runbook LTS/enterprise con SLA ejecutable:
   - script: `tools/release/runbook_lts_enterprise_sla_v1_0_1.sh`
   - validación de SLA y política de backport/soporte.

## Gate obligatorio

- `release_phase9_governance_release_gate_tests`
