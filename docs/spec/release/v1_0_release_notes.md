# nC v1.0.0 — Release notes

Fecha de publicación: 2026-04-30

## Resumen

`nC v1.0.0` queda cerrado como release mayor con continuidad tipada entre parser, AST, sema, IR y runtime/lowering/backend en las superficies oficiales.

## Incluye

- modos soberanos cerrados: `[@]`, `[@machine]`, `primitive[@]`;
- modos operativos cerrados: `[@inspect]`, `[@security]`, `[@network]`, `[@system]`, `[@tooling]`, `[@compiler]`, `[@kernel]`, `[@ai]`;
- backend material defendible y multi-target operativo;
- suite documental mayor completa;
- política de compatibilidad 1.x congelada;
- release engineering F24 cerrado con scripts y gates dedicados.

## Uso rápido

```bash
cargo test --workspace -- --nocapture
tools/release/repro_build_v1_0.sh
tools/release/smoke_v1_0.sh
```

## Referencias

- `docs/spec/release/v1_0_spec_maestra.md`
- `docs/spec/release/v1_0_acceptance_matrix.md`
- `docs/spec/release/f24_release_engineering_v1_0.md`

