# nC v1.0.0 — F15 System operativo (cierre completo)

Fecha: 2026-04-30

## Estado de cierre

`F15` queda cerrado en la línea activa para v1.0.0 con continuidad completa:

- parser;
- AST;
- sema;
- IR;
- runtime observable;
- fixtures `check/run` con matriz OK/FAIL canónica;
- documentación final de contratos.

## Surface operativa

Imports runtime soportados:

- `system.service.core.register(str) -> bool`
- `system.service.core.open(str) -> bool`
- `system.service.core.degrade(str) -> bool`
- `system.service.core.recover(str) -> bool`
- `system.service.core.reconcile() -> bool`
- `system.proc.core.spawn(str) -> bool`
- `system.proc.core.wait() -> bool`
- `system.io.core.write(str) -> bool`

Resolución de llamada:

- llamada cualificada: `system.service.core.open("auth")`;
- llamada no cualificada con import visible: `open("auth")`, `reconcile()`, etc.

## Contratos semánticos en `[@system]`

Contrato baseline (cuando hay `use system.service.core`):

- `systemOpen >> bool : ...`
- `healthy >> bool : ...`
- `degraded >> bool : ...`
- `lastEvent >> str : ...`

Contrato de flujo:

- `degrade` requiere `@systemOpen == true`;
- `recover` requiere `@degraded == true`;
- `reconcile` requiere `@systemOpen == true`.

Diagnósticos canónicos de flujo:

- `system flow violation in [@system]: 'degrade' requires open system state (expected @systemOpen == true)`
- `system flow violation in [@system]: 'recover' requires degraded state (expected @degraded == true)`
- `system flow violation in [@system]: 'reconcile' requires open system state (expected @systemOpen == true)`

## Runtime observable

Estado runtime de system:

- `system.open`
- `system.healthy`
- `system.degraded`
- `system.recovered`
- `system.events.count`
- `system.services.count`
- `system.last_event`
- `system.last_service`

Errores canónicos operativos:

- `system.service.core.register requires non-empty service:str`
- `system.service.core.open requires non-empty service:str`
- `system.service.core.open requires registered service '<name>'`
- `system.service.core.degrade requires open system state`
- `system.service.core.recover requires degraded system state`
- `system.service.core.reconcile requires open system state`

Contrato inspect:

- paths `system.*` requieren contrato de import runtime system (`system.service.core` o `system.proc.core` o `system.io.core`).

## Evidencia de cierre

### Parser/Sema/IR

`crates/ncc-parser/src/lib.rs`:

- `system_f15_tests::system_at_operational_fixtures_parse_for_runtime`

`crates/ncc-sema/src/lib.rs`:

- `system_f15_sema_tests::system_at_operational_fixtures_pass_in_sema`
- `system_f15_sema_tests::system_at_missing_baseline_stc_fails_in_sema`
- `system_f15_sema_tests::system_at_recover_without_degrade_fails_in_sema`
- `inspect_sema_tests::inspect_system_runtime_path_without_system_import_fails_in_sema`
- `inspect_sema_tests::inspect_system_runtime_path_with_system_import_passes_in_sema`

`crates/ncc-ir/src/lib.rs`:

- `system_f15_ir_tests::lower_system_fixture_lifecycle_into_typed_ir`

### Runtime/CLI

`crates/ncc-cli/src/main.rs`:

- `runtime_f15_system_tests::runtime_system_service_flow_updates_observable_state_and_inspect_paths`
- `runtime_f15_system_tests::runtime_system_service_open_requires_registered_service`
- `runtime_f15_system_fixture_matrix_tests::runtime_f15_check_ok_fixtures_run_successfully`
- `runtime_f15_system_fixture_matrix_tests::runtime_f15_runtime_failures_are_observable_with_official_diagnostics`
- `runtime_f15_system_fixture_matrix_tests::runtime_f15_check_fail_fixtures_stay_blocked_by_sema`

### Fixtures oficiales F15

`examples/system_at/`:

- check/run OK:
  - `system_at_lifecycle_ok.n`
  - `system_at_recovery_ok.n`
- check OK, run FAIL canónico:
  - `system_at_runtime_empty_service_fail.n`
- check FAIL canónico:
  - `system_at_missing_baseline_stc_fail.n`
  - `system_at_recover_without_degrade_fail.n`

## Criterio binario de F15

F15 se considera `SI` cuando:

1. lifecycle/health/degrade/recover/reconcile son ejecutables;
2. errores canónicos runtime son observables en system runtime;
3. parser/sema/IR/runtime mantienen continuidad tipada sin fallback stringly;
4. existe matriz de fixtures `check/run` con OK/FAIL canónicos;
5. documentación de contratos queda publicada y enlazada en la matriz oficial v1.0.0.
