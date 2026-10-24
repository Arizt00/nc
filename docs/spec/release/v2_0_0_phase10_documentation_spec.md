# nC Phase10 Mandatory Documentation Spec v2.0.0

Estado: `frozen`  
Fase: `phase10_p18_documentation`

## Objetivo

Cerrar F10-P18 con documentación operativa obligatoria de instalación, toolchains, IDEs, seguridad de cadena de suministro y CI/CD.

Documentos obligatorios:

1. `INSTALL.md`
2. `TOOLCHAINS.md`
3. `IDE_JETBRAINS.md`
4. `IDE_VSCODE.md`
5. `SECURITY_SUPPLY_CHAIN.md`
6. `CI_SETUP.md`

## Base activo

1. Cada documento DEBE existir en raíz de repo y ser no vacío.
2. Cada documento DEBE mantener semántica alineada con `ncup`, `ncc`, `nclsp` y `nc-sys-lsp`.
3. La documentación NO DEBE contradecir contratos `v2_0_0_phase10_*`.
4. Cualquier cambio breaking en esta familia REQUIERE RFC.

## Contenido mínimo exigido

1. `INSTALL.md`
- flujo rápido de bootstrap con `ncup-init`;
- validación de instalación con `ncup doctor`;
- ejecución de demo con `ncc run hello.n`.

2. `TOOLCHAINS.md`
- canales `stable|beta|nightly`;
- selector `channel-target`;
- componentes y perfiles `minimal|default|complete`.

3. `IDE_JETBRAINS.md`
- integración oficial con plugin nC;
- uso de `nclsp` en alto nivel;
- uso de `nc-sys-lsp` en `[@]`, `[@machine]`, `primitive[@]`.

4. `IDE_VSCODE.md`
- extensión oficial nC;
- router `nide`/LSP por modo;
- comandos de trabajo (`Run`, `Check`, `Format`, `Select Toolchain`).

5. `SECURITY_SUPPLY_CHAIN.md`
- checksum `SHA-256`;
- firma de manifests y bins;
- clave root de bootstrap y modo enterprise mirror/trust-key.

6. `CI_SETUP.md`
- acción oficial GitHub;
- template oficial GitLab;
- cache por hash de manifest y modo offline.

## Gate oficial

- `release_phase10_documentation_gate_tests`

## Pipeline de verificación

- `tools/ncc/nc/src/verification/documentation_check.n`
- `tools/release/qa_phase10_p18_v2_0_0.sh`

## Criterio de cierre

F10-P18 queda cerrado cuando:

1. spec/compat/contract P18 están en `frozen`;
2. los 6 documentos obligatorios existen y son auditables;
3. `docs/status/F10_P18_documentation_report.json` queda en `ok`;
4. el gate P18 queda integrado en QA global, contrato de Phase10 y release-train 2.x.
