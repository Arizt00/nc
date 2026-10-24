# nC v1.0.0 — F24 Release engineering (cierre operativo)

Fecha de cierre: 2026-04-30  
Estado: `SI`

## 1. Objetivo

Cerrar la salida real de `v1.0.0` como release reproducible y transferible para terceros.

## 2. Entregables obligatorios cerrados

- build reproducible;
- bundle limpio;
- artefactos oficiales;
- changelog y release notes;
- criterios de instalación;
- smoke tests finales;
- validación en WSL;
- validación en entorno limpio.

## 3. Artefactos de F24

- Guía de release engineering:
  - `docs/spec/release/f24_release_engineering_v1_0.md`
- Changelog:
  - `docs/spec/release/v1_0_changelog.md`
- Release notes:
  - `docs/spec/release/v1_0_release_notes.md`
- Manifiesto machine-readable de artefactos:
  - `docs/spec/release/v1_0_release_artifacts_manifest.json`
- Scripts oficiales:
  - `tools/release/repro_build_v1_0.sh`
  - `tools/release/smoke_v1_0.sh`

## 4. Build reproducible

Contrato mínimo:

1. `cargo test --workspace -- --nocapture` en verde;
2. artefactos de bins oficiales construibles en la misma revisión;
3. salida material verificable por checks de tamaño no-cero y comandos de smoke.

Comando de referencia:

```bash
tools/release/repro_build_v1_0.sh
```

## 5. Bundle limpio

El bundle oficial incluye:

- `crates/*` del compilador;
- `examples/*` canónicos;
- `docs/spec/release/*` (base de release);
- scripts F24 bajo `tools/release/*`.

No se consideran parte del bundle final archivos temporales ni output local de pruebas.

## 6. Smoke final de release

Se ejecuta el smoke oficial:

```bash
tools/release/smoke_v1_0.sh
```

Cobertura mínima:

- `check` + `run` de fixture `[@]`;
- `check` de fixture `[@machine]`;
- `check` de fixture `primitive[@]`;
- `emit-elf`, `emit-coff`, `emit-macho`, `build-exe`.

## 7. Validación WSL

F24 exige validar los scripts y comandos de smoke desde WSL (línea activa del proyecto).

## 8. Validación entorno limpio

Contrato operativo:

1. abrir workspace limpio;
2. ejecutar `cargo test --workspace -- --nocapture`;
3. ejecutar scripts F24 (`repro_build_v1_0.sh`, `smoke_v1_0.sh`);
4. verificar que los artefactos de salida se materializan sin ajuste manual fuera del contrato.

## 9. Gate ejecutable

Se añade gate en `ncc-cli`:

- `release_f24_release_engineering_gate_tests::*`

El gate bloquea deriva documental/contractual de F24.

## 10. Resultado

F24 queda cerrado en `SI` para `v1.0.0`, y la release mayor queda formalmente completada en esta línea.

