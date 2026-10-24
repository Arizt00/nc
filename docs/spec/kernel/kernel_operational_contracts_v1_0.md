# nC v1.0.0 — F18 Kernel profile en `.n` normal

Fecha: 2026-04-30

## Estado de cierre

`F18` queda cerrado en la línea activa para v1.0.0 con continuidad completa.
La autoridad fuente nueva vive en `.n` normal como perfil kernel implícito; `[@kernel]`
queda aceptado como compatibilidad de fixtures/proyectos antiguos, no como forma canónica.

- parser;
- AST;
- sema;
- IR;
- runtime observable;
- fixtures `check/run` con matriz OK/FAIL canónica;
- documentación final de contratos.

## Regla machine viva

Kernel no duplica machine. Cada instrucción, ABI, relocación, formato o contrato de
loader que kernel necesite y machine no soporte todavía debe cerrarse primero en
`[@machine]` con prueba de parser/sema/emit/obj/run cuando aplique. Kernel consume esa
superficie como dependencia viva.

La ruta oficial de build vive en `docs/spec/kernel/kernel_lowering_official_v1_0.md`:
`.n` normal baja con `--profile kernel` a `.nm`, `.obj` e imagen/link artifact por
target, sin requerir `[@kernel]`.

## Surface operativa

Intrínsecos runtime soportados sin import obligatorio:

- `kernel.task.core.register(int, str) -> bool`
- `kernel.schedule.core.open() -> bool`
- `kernel.schedule.core.next() -> bool`
- `kernel.control.core.quarantine(int) -> bool`
- `kernel.control.core.panic(str) -> bool`

Resolución de llamada:

- llamada cualificada: `kernel.schedule.core.open()`;
- llamada no cualificada en perfil kernel: `open()`, `next()`, `quarantine(1)`, `panic("fatal")`, `register(1, "idle")`;
- imports `use kernel.*.core` siguen aceptados como compatibilidad, pero ya no son
  necesarios para código kernel normal.

## Activación del perfil kernel

Un contenedor `.n` normal activa perfil kernel cuando declara el baseline de estado
kernel o usa llamadas cualificadas `kernel.*.core.*`. `entry[@]` con baseline kernel es
la forma preferida.

Contrato baseline:

- `schedulerOpen >> bool : ...`
- `panicState >> bool : ...`
- `quarantineCount >> int : ...`
- `lastTask >> str : ...`
- `lastEvent >> str : ...`

Contrato de flujo:

- `open` requiere estado no-pánico;
- `next` requiere estado no-pánico y scheduler abierto;
- `quarantine` requiere estado no-pánico y scheduler abierto;
- `panic` fuerza transición `panicState=true` y `schedulerOpen=false`.

Diagnósticos canónicos de flujo:

- `kernel flow violation in kernel profile: 'open' requires non-panic state (expected @panicState == false)`
- `kernel flow violation in kernel profile: 'next' requires non-panic state (expected @panicState == false)`
- `kernel flow violation in kernel profile: 'next' requires open scheduler state (expected @schedulerOpen == true)`
- `kernel flow violation in kernel profile: 'quarantine' requires non-panic state (expected @panicState == false)`
- `kernel flow violation in kernel profile: 'quarantine' requires open scheduler state (expected @schedulerOpen == true)`

## Runtime observable

Estado runtime de kernel:

- `kernel.scheduler.open`
- `kernel.panic.state`
- `kernel.quarantine.count`
- `kernel.events.count`
- `kernel.last_task`
- `kernel.last_event`

Errores canónicos operativos:

- `kernel.task.core.register requires pid:int`
- `kernel.task.core.register requires pid > 0`
- `kernel.task.core.register requires non-empty task:str`
- `kernel.task.core.register requires non-panic kernel state`
- `kernel.schedule.core.open requires non-panic kernel state`
- `kernel.schedule.core.next requires non-panic state`
- `kernel.schedule.core.next requires open scheduler state`
- `kernel.control.core.quarantine requires pid:int`
- `kernel.control.core.quarantine requires pid > 0`
- `kernel.control.core.quarantine requires non-panic state`
- `kernel.control.core.quarantine requires open scheduler state`
- `kernel.control.core.panic requires non-empty reason:str`

Contrato inspect:

- paths `kernel.*` son intrínsecos de runtime kernel y no requieren import.

## Evidencia de cierre

### Parser/Sema/IR

`crates/ncc-parser/src/lib.rs`:

- `kernel_f18_tests::kernel_at_operational_fixtures_parse_for_runtime`
- `kernel_f18_tests::kernel_at_invalid_section_fails_in_parser`

`crates/ncc-sema/src/lib.rs`:

- `kernel_f18_sema_tests::kernel_at_operational_fixtures_pass_in_sema`
- `kernel_f18_sema_tests::kernel_at_missing_baseline_stc_fails_in_sema`
- `kernel_f18_sema_tests::kernel_at_schedule_without_open_fails_in_sema`
- `kernel_f18_sema_tests::kernel_at_quarantine_after_panic_fails_in_sema`
- `kernel_f18_sema_tests::kernel_profile_normal_n_without_imports_passes_in_sema`
- `inspect_sema_tests::inspect_kernel_runtime_path_without_kernel_import_passes_in_sema`
- `inspect_sema_tests::inspect_kernel_runtime_path_with_kernel_import_passes_in_sema`

`crates/ncc-ir/src/lib.rs`:

- `kernel_f18_ir_tests::lower_kernel_fixture_lifecycle_into_typed_ir`

### Runtime/CLI

`crates/ncc-cli/src/main.rs`:

- `runtime_f10_import_tests::runtime_kernel_import_surface_executes`
- `runtime_f18_kernel_tests::runtime_kernel_flow_updates_observable_state_and_inspect_paths`
- `runtime_f18_kernel_tests::runtime_kernel_panic_disables_scheduler_and_blocks_next`
- `runtime_f18_kernel_tests::runtime_kernel_profile_normal_n_without_imports_executes`
- `runtime_f18_kernel_fixture_matrix_tests::runtime_f18_check_ok_fixtures_run_successfully`
- `runtime_f18_kernel_fixture_matrix_tests::runtime_f18_runtime_failures_are_observable_with_official_diagnostics`
- `runtime_f18_kernel_fixture_matrix_tests::runtime_f18_check_fail_fixtures_stay_blocked_by_sema`

### Fixtures oficiales F18

`examples/kernel_at/`:

- check/run OK:
  - `kernel_at_lifecycle_ok.n`
  - `kernel_at_alias_family_ok.n`
  - `kernel_at_runtime_inspect_paths_ok.n`
- check OK, run FAIL canónico:
  - `kernel_at_runtime_empty_task_fail.n`
- check FAIL canónico:
  - `kernel_at_missing_baseline_stc_fail.n`
  - `kernel_at_schedule_without_open_fail.n`
  - `kernel_at_quarantine_after_panic_fail.n`
  - `kernel_at_invalid_section_fail.n` (bloqueado en parser por surface modal)

## Criterio binario de F18

F18 se considera `SI` cuando:

1. modelo mínimo de tareas/eventos del kernel es ejecutable en runtime;
2. flujo de scheduler y panic/quarantine mantiene contratos tipados en sema;
3. inspect kernel expone estado operativo observable y auditabilidad de eventos;
4. parser/sema/IR/runtime mantienen continuidad tipada sin fallback stringly;
5. existe matriz de fixtures `check/run` con OK/FAIL canónicos;
6. documentación de contratos queda publicada y enlazada en la matriz oficial v1.0.0.
