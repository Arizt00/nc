# nC v1.0.0 — F14 Network ejecutable (cierre completo)

Fecha: 2026-04-30

## Estado de cierre

`F14` queda cerrado en la línea activa para v1.0.0 con continuidad completa:

- parser;
- AST;
- sema;
- IR;
- runtime observable;
- fixtures `check/run` con matriz OK/FAIL canónica;
- documentación final de contratos.

## Surface operativa

Imports runtime soportados:

- `system.net.core.connect(str, int) -> bool`
- `system.net.core.listen() -> bool`
- `system.net.core.accept() -> bool`
- `system.net.core.send(str) -> bool`
- `system.net.core.recv() -> bool`

Alias compatible:

- `alt.net.core.*`

Resolución de llamada:

- llamada cualificada: `system.net.core.connect(...)`;
- llamada no cualificada con import visible: `connect(...)`, `listen(...)`, `accept(...)`, `send(...)`, `recv(...)`.

## Contratos semánticos en `[@network]`

Contrato estructural:

- `final` obligatorio y no vacío;
- cierre modal de secciones no permitidas con diagnóstico canónico.

Contrato baseline cuando hay superficie runtime de red:

- `connected >> bool : ...`
- `listening >> bool : ...`
- `lastError >> str : ...`

Contrato de flujo:

- `accept` requiere `@listening == true`;
- `send` requiere `@connected == true`;
- `recv` requiere `@connected == true`;
- `connect` y `listen` actualizan estado de flujo en análisis.

## Runtime observable

Estado runtime de red:

- `net.connected`
- `net.listening`
- `net.accepted`
- `net.last_send`
- `net.host`
- `net.port`
- `net.last_payload`

Errores canónicos operativos:

- `system.net.core.connect requires valid port > 0`
- `system.net.core.send requires active connection`
- `system.net.core.recv requires active connection`
- `system.net.core.accept requires listening socket`

## Evidencia de cierre

### Parser/Sema/IR

`crates/ncc-parser/src/lib.rs`:

- `network_f9_tests::network_at_minimal_ok_parses`
- `network_f9_tests::network_at_flow_contract_ok_parses`
- `network_f9_tests::network_at_runtime_bad_port_fail_parses_for_runtime`
- `network_f9_tests::network_at_invalid_section_fails_in_parser`

`crates/ncc-sema/src/lib.rs`:

- `network_f9_sema_tests::network_at_minimal_ok_passes_in_sema`
- `network_f9_sema_tests::network_at_flow_contract_ok_passes_in_sema`
- `network_f9_sema_tests::network_at_runtime_bad_port_fail_passes_in_sema_before_runtime`
- `network_f9_sema_tests::network_at_missing_baseline_stc_fails_in_sema`
- `network_f9_sema_tests::network_at_send_without_connection_fails_in_sema`

`crates/ncc-ir/src/lib.rs`:

- `network_ir_tests::*` (lowering de red tipado)

### Runtime/CLI

`crates/ncc-cli/src/main.rs`:

- `runtime_import_alt_net_tests::runtime_alt_net_connect_send_ok`
- `runtime_import_alt_net_tests::runtime_alt_net_send_without_connect_fails_with_alt_path`
- `runtime_import_alt_net_tests::runtime_unqualified_network_calls_resolve_from_visible_import`
- `runtime_f14_network_fixture_matrix_tests::runtime_f14_check_ok_fixtures_run_successfully`
- `runtime_f14_network_fixture_matrix_tests::runtime_f14_runtime_failures_are_observable_with_official_diagnostics`
- `runtime_f14_network_fixture_matrix_tests::runtime_f14_check_fail_fixtures_stay_blocked_by_sema`

### Fixtures oficiales F14

`examples/network_at/`:

- check/run OK:
  - `network_at_flow_contract_ok.n`
- check OK, run FAIL canónico:
  - `network_at_runtime_bad_port_fail.n`
- check FAIL canónico:
  - `network_at_missing_baseline_stc_fail.n`
  - `network_at_send_without_connection_fail.n`

## Criterio binario de F14

F14 se considera `SI` cuando:

1. `connect/listen/accept/send/recv` son ejecutables con estado coherente;
2. errores canónicos de red son observables en runtime;
3. parser/sema/IR/runtime mantienen continuidad tipada sin fallback stringly;
4. existe matriz de fixtures `check/run` con OK/FAIL canónicos;
5. documentación de contratos queda publicada y enlazada en la matriz oficial v1.0.0.
