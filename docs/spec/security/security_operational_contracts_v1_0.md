# nC v1.0.0 — F13 Security operativo (cierre completo)

Fecha: 2026-04-30

## Estado de cierre

`F13` queda cerrado en la línea activa para v1.0.0 con continuidad completa:

- parser;
- AST;
- sema;
- IR;
- runtime observable;
- fixtures `check/run` con matriz OK/FAIL canónica;
- documentación final de contratos.

No degrada la arquitectura tipada existente.

## Surface operativa y contratos

Módulos soportados en runtime:

- `security.policy.strict.evaluate(action) -> bool`
- `security.intel.attack.simulate(str) -> bool`

Contratos:

- `evaluate` acepta decisiones canónicas: `allow`, `deny`, `quarantine`, `escalate`.
- `simulate` requiere `allowOffensive == true` en estado activo (guardia runtime de SEC047).

## Modelo operativo de decisiones

Decisiones activas:

- `allow`
- `deny`
- `quarantine`
- `escalate`

Reglas materiales:

- `allow`: permite continuidad del flujo.
- `deny`: bloquea flujo de red activo.
- `quarantine`: bloquea flujo de red activo y deja traza de cuarentena.
- `escalate`: bloquea flujo de red activo y deja traza de escalamiento.
- `attack.simulate`: exige autorización ofensiva (`allowOffensive`).

## Estado observable (runtime)

Estado agregado:

- `security.last_action`
- `security.allow.count`
- `security.deny.count`
- `security.quarantine.count`
- `security.escalate.count`
- `security.audit.count`
- `security.attack.count`
- `security.last_attack`

Reglas de efecto:

- `security.policy.strict.evaluate(...)`:
  - actualiza `last_action`;
  - incrementa contador por decisión;
  - incrementa `audit.count` cuando `audit` está habilitado;
  - para `deny/quarantine/escalate`, cierra conexión de red activa (`net.connected=false`, `net.accepted=false`, `net.last_send=false`).
- `security.intel.attack.simulate(...)`:
  - valida guardia `allowOffensive`;
  - incrementa `attack.count`;
  - actualiza `last_attack`;
  - incrementa `audit.count` si `audit` está habilitado.

## Integración con inspect/runtime/network

Paths canónicos nuevos:

- `security.last_action`
- `security.allow.count`
- `security.deny.count`
- `security.quarantine.count`
- `security.escalate.count`
- `security.audit.count`
- `security.attack.count`
- `security.last_attack`

Regla semántica:

- estos paths requieren contrato importado de seguridad:
  - `use security.policy.strict;` o
  - `use security.intel.attack;`

Efecto de enforcement sobre red:

- tras `deny`, `quarantine` o `escalate`:
  - `net.connected = false`
  - `net.accepted = false`
  - `net.last_send = false`
- cualquier `system.net.core.send(...)` posterior sin reconexión válida falla con diagnóstico runtime canónico:
  - `system.net.core.send requires active connection`

## Evidencia de cierre

### Parser/Sema/IR

`crates/ncc-cli/src/main.rs`:

- `runtime_f13_security_tests::runtime_security_policy_evaluate_tracks_decision_and_audit_state`
- `runtime_f13_security_tests::runtime_security_attack_simulate_tracks_attack_and_audit_state`
- `runtime_f13_security_tests::runtime_security_deny_enforcement_blocks_subsequent_network_send`
- `runtime_f13_security_tests::runtime_security_quarantine_and_escalate_update_counters_and_enforce_cutoff`
- `runtime_f13_security_tests::runtime_security_inspect_paths_render_observable_state`
- `runtime_f13_security_fixture_matrix_tests::runtime_f13_check_ok_fixtures_run_successfully`
- `runtime_f13_security_fixture_matrix_tests::runtime_f13_enforcement_failures_are_observable_in_runtime`
- `runtime_f13_security_fixture_matrix_tests::runtime_f13_check_fail_fixtures_stay_blocked_by_sema`

`crates/ncc-sema/src/lib.rs`:

- `inspect_sema_tests::inspect_security_runtime_path_without_security_import_fails_in_sema`
- `inspect_sema_tests::inspect_security_runtime_path_with_security_import_passes_in_sema`
- `security_f8_sema_tests::security_at_runtime_allow_ok_passes_in_sema`
- `security_f8_sema_tests::security_at_runtime_deny_block_fixture_passes_in_sema`
- `security_f8_sema_tests::security_at_inspect_contract_missing_import_fails_in_sema`

`crates/ncc-ast/src/lib.rs`:

- `inspect_runtime_path_ast_tests::inspect_runtime_path_from_segments_maps_official_paths`
- `inspect_runtime_path_ast_tests::inspect_runtime_path_from_segments_rejects_unknown_paths`

`crates/ncc-parser/src/lib.rs`:

- `security_f8_tests::security_at_runtime_allow_ok_parses`
- `security_f8_tests::security_at_runtime_inspect_paths_ok_parses`
- `security_f8_tests::security_at_inspect_contract_missing_import_parses_for_sema`

### Fixtures oficiales F13

`examples/security_at/`:

- check/run OK:
  - `security_at_runtime_allow_ok.n`
  - `security_at_runtime_inspect_paths_ok.n`
- check OK, run FAIL canónico (enforcement):
  - `security_at_runtime_deny_blocks_send_fail.n`
  - `security_at_runtime_quarantine_blocks_send_fail.n`
  - `security_at_runtime_escalate_blocks_send_fail.n`
- check FAIL canónico:
  - `security_at_attack_import_without_lab_fail.n` (`SEC047`)
  - `security_at_inspect_contract_missing_import_fail.n` (falta contrato inspect/security)

Verificación global:

```bash
cargo test --workspace -- --nocapture
```

## Criterio binario de F13

F13 se considera `SI` cuando:

1. decisiones `allow/deny/quarantine/escalate` son ejecutables;
2. enforcement de red es observable y canónico;
3. paths `inspect` de seguridad están tipados y con contrato de import;
4. existen fixtures `check/run` en matriz OK/FAIL;
5. parser/AST/sema/IR/runtime permanecen tipados sin fallback stringly.
