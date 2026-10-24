# nC Phase10 Documentation Compatibility Spec v2.0.0

Estado: `frozen`  
Serie: `2.x`

## Compatibilidad contractual

1. La documentación obligatoria DEBE permanecer estable en nombres:
- `INSTALL.md`
- `TOOLCHAINS.md`
- `IDE_JETBRAINS.md`
- `IDE_VSCODE.md`
- `SECURITY_SUPPLY_CHAIN.md`
- `CI_SETUP.md`

2. El contenido DEBE evolucionar de forma aditiva en series 2.x.
3. Renombres o retirada de estos documentos PROHÍBE promoción de canal sin RFC.

## Compatibilidad de semántica

1. El compilador (`ncc`) sigue siendo autoridad semántica.
2. Los documentos de IDE NO DEBEN elevar el editor a autoridad del lenguaje.
3. La documentación de seguridad DEBE reflejar verificación obligatoria preinstalación.
4. La documentación CI DEBE mantener coherencia con scripts oficiales en `tools/ci`.

## Gate obligatorio

`release_phase10_documentation_gate_tests` DEBE validar esta compatibilidad en cada promoción 2.x.
