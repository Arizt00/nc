# Feature Test Suite Contract

Este documento cierra el contrato de pruebas por feature para nC.

Una feature nueva no se considera cerrada solo porque el archivo compile. Para poder marcarla como cerrada debe tener siete carriles verificables:

1. `example_n`: un ejemplo `.n` positivo y ejecutable.
2. `parser`: un test que pase por parser/AST.
3. `sema`: un test que pase por analisis semantico.
4. `ir`: un test que demuestre lowering a IR.
5. `mct`: un test que demuestre materializacion MCT/backend cuando aplique.
6. `run`: un test de ejecucion real.
7. `negative`: un ejemplo negativo con diagnostico esperado.

El contrato machine-readable vive en:

```bash
ncc compiler feature-suite --json --strict
```

`--strict` falla si una suite registrada apunta a un fixture, test o documento inexistente, o si su carril negativo no declara diagnostico esperado.

## Suite Inicial Cerrada

La primera suite registrada es `compact_post_increment`.

- ejemplo positivo: `examples/postIncrementCompact.n`
- fixture MCT: `examples/backend/mct_llvm_post_increment.n`
- ejemplo negativo: `examples/feature_suites/negative/compact_post_increment_unbound_fail.n`
- diagnostico negativo esperado: `E_HIGH_POST_INCREMENT_TARGET_001`
- tests: `crates/ncc-cli/tests/feature_suite_contract.rs`

Esta suite prueba la misma feature desde parser, sema, IR, MCT, ejecucion y fallo semantico. A partir de aqui, cualquier feature nueva debe agregar una fila equivalente al contrato antes de considerarse cerrada.
