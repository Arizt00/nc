# nC v1.0.0 - Compiler operativo

Fecha: 2026-04-30

## Autoridad actual

La ABI actual es `nC.runtime.compilerManual`: 21 operaciones import-free sobre
un workspace raw de 128 bytes y tablas aportadas por el llamador. Parse, AST,
sema, lowering, diagnosticos, report y manifest se materializan dentro de
LLVM/MCT/Machine sin proveedor C ni asignacion oculta.

Las llamadas `compiler.parse.ast.load(str)`,
`compiler.semantic.core.check(str)` y
`compiler.backend.machine.materialize(str)` descritas mas abajo son la surface
modal heredada. Se conservan por compatibilidad, pero no son la ABI manual
canonica para codigo nuevo.

## Estado de compatibilidad F17

`F17` queda cerrado en la línea activa para v1.0.0 con continuidad completa:

- parser;
- AST;
- sema;
- IR;
- runtime observable;
- fixtures `check/run` con matriz OK/FAIL canónica;
- documentación final de contratos.

## Surface operativa

Imports runtime soportados:

- `compiler.parse.ast.load(str) -> bool`
- `compiler.semantic.core.check(str) -> bool`
- `compiler.backend.machine.materialize(str) -> bool`

Alias compatible de familias:

- `use nC.compiler.parse;`
- `use nC.compiler.semantic;`
- `use nC.compiler.backend;`
- `use nC.compiler.*;`

## Contratos semánticos en `[@compiler]`

Contrato baseline (cuando hay surface runtime compiler):

- `parseReady >> bool : ...`
- `semaReady >> bool : ...`
- `backendReady >> bool : ...`
- `lastUnit >> str : ...`

Contrato de flujo:

- `check` requiere `@parseReady == true`.
- `materialize` requiere `@semaReady == true`.

Diagnósticos canónicos de flujo:

- `compiler flow violation in [@compiler]: 'check' requires parse-ready state (expected @parseReady == true)`
- `compiler flow violation in [@compiler]: 'materialize' requires sema-ready state (expected @semaReady == true)`

## Runtime observable

Estado runtime de compiler:

- `compiler.parse.ready`
- `compiler.sema.ready`
- `compiler.backend.ready`
- `compiler.events.count`
- `compiler.last_unit`

Errores canónicos operativos:

- `compiler.parse.ast.load requires non-empty unit:str`
- `compiler.semantic.core.check requires non-empty unit:str`
- `compiler.semantic.core.check requires parse-ready unit state`
- `compiler.backend.machine.materialize requires non-empty unit:str`
- `compiler.backend.machine.materialize requires sema-ready unit state`

Contrato inspect:

- paths `compiler.*` requieren contrato importado de compiler runtime.

## Evidencia de cierre

`crates/ncc-parser/src/lib.rs`:

- `compiler_f17_tests::compiler_at_operational_fixtures_parse_for_runtime`
- `compiler_f17_tests::compiler_at_invalid_section_fails_in_parser`

`crates/ncc-sema/src/lib.rs`:

- `compiler_f17_sema_tests::compiler_at_operational_fixtures_pass_in_sema`
- `compiler_f17_sema_tests::compiler_at_missing_baseline_stc_fails_in_sema`
- `compiler_f17_sema_tests::compiler_at_materialize_without_check_fails_in_sema`
- `inspect_f7_sema_tests::inspect_compiler_runtime_path_without_compiler_import_fails_in_sema`
- `inspect_f7_sema_tests::inspect_compiler_runtime_path_with_compiler_import_passes_in_sema`

`crates/ncc-ir/src/lib.rs`:

- `compiler_f17_ir_tests::lower_compiler_fixture_lifecycle_into_typed_ir`

`crates/ncc-cli/src/main.rs`:

- `runtime_f17_compiler_tests::runtime_compiler_flow_updates_observable_state_and_inspect_paths`
- `runtime_f17_compiler_fixture_matrix_tests::runtime_f17_check_ok_fixtures_run_successfully`
- `runtime_f17_compiler_fixture_matrix_tests::runtime_f17_runtime_failures_are_observable_with_official_diagnostics`
- `runtime_f17_compiler_fixture_matrix_tests::runtime_f17_check_fail_fixtures_stay_blocked_by_sema`

## Fixtures oficiales F17

`examples/compiler_at/`:

- check/run OK:
  - `compiler_at_lifecycle_ok.n`
  - `compiler_at_alias_family_ok.n`
  - `compiler_at_runtime_inspect_paths_ok.n`
- check OK, run FAIL canónico:
  - `compiler_at_runtime_empty_unit_fail.n`
- check FAIL canónico:
  - `compiler_at_missing_baseline_stc_fail.n`
  - `compiler_at_materialize_without_check_fail.n`

## Frontera oficial de F17

`[@compiler]` queda definido como capa de coordinación de pipeline semántico/IR/lowering, no como sustituto de `[@tooling]`:

- `[@compiler]`: estados y contratos de `load/check/materialize`.
- `[@tooling]`: empaquetado/build/release (`plan/apply/pack/publish`) y comandos ecosistema (`nfmt/nlint/nbuild/npkg`).
- `F0/F11`: proveen infraestructura de compilador/backend; `[@compiler]` la expone de forma modal tipada y observable.

## Criterio binario de F17

F17 se considera `SI` cuando:

1. surface `compiler.parse/semantic/backend` es ejecutable con estado coherente;
2. errores canónicos runtime son observables;
3. baseline/flow semántico en `[@compiler]` queda tipado y estable;
4. existe matriz de fixtures `check/run` con OK/FAIL canónicos;
5. la frontera `[@compiler]` vs `[@tooling]` queda explícita y documentada.
