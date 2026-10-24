# nC v1.0.0 — Handoff para terceros

## 1. Entrada rápida

1. Ejecutar tests globales.
2. Leer roadmap obligatorio y matriz de aceptación.
3. Validar un fixture por modo con `check` y `run`.

## 2. Documentos de arranque

- `docs/spec/release/v1_0_spec_maestra.md`
- `docs/spec/release/v1_0_roadmap_obligatorio.md`
- `docs/spec/release/v1_0_acceptance_matrix.md`
- `docs/spec/release/v1_0_compatibility_policy_1x.md`
- `docs/spec/release/f24_release_engineering_v1_0.md`
- `docs/handoff/ROADMAP_CODEX_MASTER.md`

## 3. Política de intervención

- No romper continuidad tipada parser -> AST -> sema -> IR -> runtime/backend.
- No introducir fallback textual en zonas tipadas cerradas.
- Toda modificación debe venir acompañada de test y evidencia documental.

## 4. Checklist mínimo para PR técnico

- test unitario/integración añadido o ajustado;
- diagnóstico canónico preservado/actualizado;
- actualización de documentación correspondiente;
- `cargo test --workspace -- --nocapture` en verde.
