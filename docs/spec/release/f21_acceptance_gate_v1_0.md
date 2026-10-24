# nC v1.0.0 — F21 Matriz oficial de aceptación (cierre operativo)

Fecha de cierre: 2026-04-30

## 1. Objetivo de F21

F21 exige un criterio objetivo y reproducible de `done` por modo y por capa.
No es una opinión: es una matriz ejecutable con evidencia verificable.

## 2. Criterio estricto de cierre

F21 se considera cerrado solo si:

1. existe matriz oficial por modo/capa (`docs/spec/release/v1_0_acceptance_matrix.md`);
2. hay evidencia ejecutable para casos `OK` y `FAIL` canónicos;
3. la matriz no deriva de la implementación (doc/test sincronizados);
4. el workspace completo se mantiene en verde.

## 3. Gate ejecutable añadido (F21)

Se añadió una puerta dedicada en `crates/ncc-cli/src/main.rs`:

- `release_f21_acceptance_gate_tests::f21_mode_matrix_ok_cases_cover_active_modes_end_to_end`
- `release_f21_acceptance_gate_tests::f21_mode_matrix_fail_cases_keep_official_diagnostics`
- `release_f21_acceptance_gate_tests::f21_release_matrix_document_tracks_closed_rows_as_si`

Cobertura activa en el gate:

- `[@]`
- `[@machine]`
- `primitive[@]`
- Linking soberano
- `[@inspect]`
- `[@security]`
- `[@network]`
- `[@system]`
- `[@tooling]`
- `[@compiler]`
- `[@kernel]`
- `[@ai]`

Para cada modo se valida continuidad tipada:

- parser
- sema
- lowering/backend/object/ELF64 material
- runtime `run_main` cuando aplica
- diagnósticos canónicos en fixtures `FAIL`

## 4. Estado oficial de la matriz tras el cierre F21

En `v1_0_acceptance_matrix.md` quedan en `SI` total:

- `[@]` (F1 endurecido)
- `[@machine]` (F2 expandido)
- Linking soberano (F4)
- Backend material defendible (F20)

Y la matriz queda coherente con cierre total de fases de release mayor.

## 5. Comandos de verificación reproducible

```bash
cargo test -p ncc-cli release_f21_acceptance_gate_tests:: -- --nocapture
cargo test --workspace -- --nocapture
```

## 6. Resultado binario

F21 queda `SI` en línea activa v1.0.0:

- matriz oficial presente,
- validación ejecutable activa,
- evidencia reproducible,
- coherencia doc <-> implementación.
