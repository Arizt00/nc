# nC v1.0.0 — F16 Tooling operativo (cierre completo)

Fecha: 2026-04-30

## Estado de cierre

`F16` queda cerrado en la línea activa para v1.0.0 con continuidad completa:

- parser;
- AST;
- sema;
- IR;
- runtime observable;
- fixtures `check/run` con matriz OK/FAIL canónica;
- comandos ecosistema ejecutables (`nfmt`, `nlint`, `nbuild`, `npkg`);
- documentación final de contratos.

## Surface operativa

Imports runtime soportados:

- `tooling.build.core.plan(str) -> bool`
- `tooling.build.core.apply(str) -> bool`
- `tooling.lint.core.run(str) -> bool`
- `tooling.fmt.core.run(str) -> bool`
- `tooling.pkg.core.pack(str) -> bool`
- `tooling.pkg.core.publish(str) -> bool`

Alias compatible de familias:

- `use nC.tooling.build;`
- `use nC.tooling.lint;`
- `use nC.tooling.fmt;`
- `use nC.tooling.pkg;`
- `use nC.tooling.*;`

## Contratos semánticos en `[@tooling]`

Contrato baseline (cuando hay surface runtime tooling):

- `buildReady >> bool : ...`
- `lintReady >> bool : ...`
- `formatReady >> bool : ...`
- `packageReady >> bool : ...`
- `publishReady >> bool : ...`
- `lastArtifact >> str : ...`

Contrato de flujo:

- `apply` requiere `@buildReady == true`.
- `publish` requiere `@packageReady == true`.

Diagnósticos canónicos de flujo:

- `tooling flow violation in [@tooling]: 'apply' requires build-ready state (expected @buildReady == true)`
- `tooling flow violation in [@tooling]: 'publish' requires packaged artifact state (expected @packageReady == true)`

## Runtime observable

Estado runtime de tooling:

- `tooling.build.ready`
- `tooling.lint.ready`
- `tooling.format.ready`
- `tooling.package.ready`
- `tooling.publish.ready`
- `tooling.events.count`
- `tooling.last_target`

Errores canónicos operativos:

- `tooling.build.core.plan requires non-empty target:str`
- `tooling.build.core.apply requires non-empty target:str`
- `tooling.build.core.apply requires planned build state`
- `tooling.lint.core.run requires non-empty scope:str`
- `tooling.fmt.core.run requires non-empty scope:str`
- `tooling.pkg.core.pack requires non-empty target:str`
- `tooling.pkg.core.publish requires non-empty target:str`
- `tooling.pkg.core.publish requires packaged artifact state`

Contrato inspect:

- paths `tooling.*` requieren contrato importado de tooling runtime.

## Tooling de ecosistema (CLI)

Comandos activos en `ncc-cli`:

- `nfmt <file.n> [--check|--compact|--in-place|--out <path>]`
- `nlint <file.n>`
- `nbuild <file.n> [out-exe] [--out-dir <dir>] [--json] [--cc <nC.sysv64|nC.win64|nC.aapcs64>] [--linker <bin>] [--entry <symbol>]`
- `npkg <manifest|check-imports|list-imports> <file.n> [--out <path>]`

Binaries dedicados:

- `ncc`
- `nfmt`
- `nlint`
- `nbuild`
- `npkg`

## Evidencia de cierre

`crates/ncc-parser/src/lib.rs`:

- `tooling_f16_tests::tooling_at_operational_fixtures_parse_for_runtime`
- `tooling_f16_tests::tooling_at_invalid_section_fails_in_parser`

`crates/ncc-sema/src/lib.rs`:

- `tooling_f16_sema_tests::tooling_at_operational_fixtures_pass_in_sema`
- `tooling_f16_sema_tests::tooling_at_missing_baseline_stc_fails_in_sema`
- `tooling_f16_sema_tests::tooling_at_publish_without_pack_fails_in_sema`
- `inspect_sema_tests::inspect_tooling_runtime_path_without_tooling_import_fails_in_sema`
- `inspect_sema_tests::inspect_tooling_runtime_path_with_tooling_import_passes_in_sema`

`crates/ncc-ir/src/lib.rs`:

- `tooling_f16_ir_tests::lower_tooling_fixture_lifecycle_into_typed_ir`

`crates/ncc-cli/src/main.rs`:

- `runtime_f10_import_tests::runtime_tooling_import_surface_executes`
- `runtime_f16_tooling_tests::runtime_tooling_flow_updates_observable_state_and_inspect_paths`
- `runtime_f16_tooling_fixture_matrix_tests::runtime_f16_check_ok_fixtures_run_successfully`
- `runtime_f16_tooling_fixture_matrix_tests::runtime_f16_runtime_failures_are_observable_with_official_diagnostics`
- `runtime_f16_tooling_fixture_matrix_tests::runtime_f16_check_fail_fixtures_stay_blocked_by_sema`

## Fixtures oficiales F16

`examples/tooling_at/`:

- check/run OK:
  - `tooling_at_lifecycle_ok.n`
  - `tooling_at_runtime_inspect_paths_ok.n`
- check OK, run FAIL canónico:
  - `tooling_at_runtime_empty_target_fail.n`
- check FAIL canónico:
  - `tooling_at_missing_baseline_stc_fail.n`
  - `tooling_at_publish_without_pack_fail.n`

## Criterio binario de F16

F16 se considera `SI` cuando:

1. surface `tooling.build/lint/fmt/pkg` es ejecutable con estado coherente;
2. errores canónicos runtime son observables;
3. baseline/flow semántico en `[@tooling]` queda tipado y estable;
4. existe matriz de fixtures `check/run` con OK/FAIL canónicos;
5. `nfmt/nlint/nbuild/npkg` forman parte del flujo oficial de tooling.
