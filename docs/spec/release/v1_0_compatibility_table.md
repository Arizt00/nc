# nC v1.0.0 — Tabla de compatibilidad

## 1. Compatibilidad de superficie

| Área | Estado |
|---|---|
| Superficies cerradas F12..F24 | Compatible en línea activa |
| Diagnósticos canónicos cerrados | Compatible (sin drift intencional) |
| Fixtures canónicos `OK/FAIL` | Compatibles con gate de aceptación |

## 2. Compatibilidad de backend/artefacto

| Artefacto | Perfiles compatibles |
|---|---|
| `emit-elf` | `nC.sysv64`, `nC.aapcs64` |
| `emit-coff` | perfiles activos con ruta tipada/fallback controlado |
| `emit-macho` | `nC.sysv64`, `nC.aapcs64` |
| `build-exe` | `nC.sysv64`, `nC.aapcs64` |

## 3. Política 1.x congelada

La política formal de compatibilidad 1.x queda cerrada en:

- `docs/spec/release/v1_0_compatibility_policy_1x.md`
- `docs/spec/release/v1_0_compatibility_contract.json`
