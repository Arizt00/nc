# nC Phase10 CI/CD Compatibility Spec v2.0.0

Estado: `frozen`  
Fase: `phase10_p14_cicd_compat`

## Objetivo

Cerrar F10-P14 con compatibilidad CI/CD productiva para GitHub y GitLab:

1. instalación oficial de `channel + target + components`;
2. cache de toolchains determinada por `sha256(manifest)`;
3. modo offline con artifacts predescargados;
4. integración sin romper autoridad semántica del compilador.

## Artefactos canónicos

- `tools/ci/setup_ncup_ci_v2_0_0.sh`
- `tools/ci/github/setup_ncup_ci_v2_0_0.sh`
- `tools/ci/gitlab/setup_ncup_ci_v2_0_0.sh`
- `.github/actions/setup-nc/action.yml`
- `.gitlab-ci.ncup.yml`

## Reglas canónicas

1. El setup CI DEBE resolver toolchain explícita (`<channel>-<target>`) y validar su existencia con `phase10-toolchain-model`.
2. El setup CI DEBE aceptar componentes explícitos (`--components`) y aplicarlos vía `ncup component add`.
3. El cache key DEBE incluir hash SHA-256 del manifest de release.
4. El setup CI DEBE soportar restore/save de cache en ruta configurable (`--cache-root`).
5. El modo offline DEBE aceptar bundle predescargado (`--offline --offline-bundle`) y completar setup sin fetch de red.
6. GitHub y GitLab DEBEN exponer integración oficial reutilizable (action/template) sin redefinir semántica del lenguaje.
7. `ncc` DEBE seguir siendo autoridad semántica; la automatización CI NO DEBE introducir reglas de análisis paralelas.

## Gate oficial

- `release_phase10_cicd_compat_gate_tests`

## Pipeline de verificación

- `ncc-release-v43 release-cicd-compat` ([build V43](../../NATIVE_TOOLS_V43_COMMANDS.md))
- `tools/release/qa_phase10_p14_v2_0_0.sh`

## Criterio de cierre

F10-P14 queda cerrado cuando:

1. spec/compat/contract P14 están en `frozen`;
2. setup CI instala canal/target/componentes de forma determinista;
3. ejecución consecutiva demuestra cache `miss -> hit` por hash de manifest;
4. modo offline funciona con bundle predescargado;
5. reporte `docs/status/F10_P14_cicd_compat_report.json` queda en `ok`;
6. gate P14 está integrado en QA global y release-train 2.x.
