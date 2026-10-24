# nC v1.0.0 — F19 AI operativo (cierre completo)

Fecha: 2026-04-30

## Estado de cierre

`F19` queda cerrado en la línea activa para v1.0.0 con continuidad completa:

- parser;
- AST;
- sema;
- IR;
- runtime observable;
- fixtures `check/run` con matriz OK/FAIL canónica;
- documentación final de contratos.

## Decisión oficial F19 (v1.0.0)

F19 entra como **capa operativa completa de orquestación IA tipada** para construir agentes y asistentes con contrato estricto.

Alcance explícito de v1.0.0:

- sí incluye surface, sema, IR y runtime de sesión/modelo/planificación/ejecución/chat/memoria;
- sí permite integración canónica con proveedores (`openai`, `anthropic`, `google`) a nivel de contrato de runtime;
- no pretende entrenar modelos fundacionales dentro del compilador en esta versión.

## Surface operativa

Imports runtime soportados:

- `ai.session.core.open(str) -> bool`
- `ai.session.core.close() -> bool`
- `ai.model.core.select(str, str) -> bool`
- `ai.agent.core.plan(str) -> bool`
- `ai.agent.core.critique(str) -> bool`
- `ai.agent.core.execute(str) -> bool`
- `ai.chat.core.complete(str) -> str`
- `ai.memory.core.remember(str, str) -> bool`
- `ai.memory.core.recall(str) -> str`

Resolución de llamada:

- llamada cualificada: `ai.chat.core.complete("hola")`;
- llamada no cualificada con import visible: `open(...)`, `select(...)`, `plan(...)`, `complete(...)`, etc.

## Contratos semánticos en `[@ai]`

Contrato baseline (cuando hay `use ai.*.core`):

- `sessionActive >> bool : ...`
- `modelReady >> bool : ...`
- `plannerReady >> bool : ...`
- `criticReady >> bool : ...`
- `executorReady >> bool : ...`
- `memoryCount >> int : ...`
- `lastProvider >> str : ...`
- `lastModel >> str : ...`
- `lastGoal >> str : ...`
- `lastPrompt >> str : ...`
- `lastResponse >> str : ...`

Contrato de flujo:

- `select` requiere sesión activa;
- `plan` requiere sesión activa y modelo listo;
- `critique` requiere planner listo;
- `execute` requiere planner y critic listos;
- `complete` requiere sesión activa y modelo listo;
- `remember/recall` requieren sesión activa.

Diagnósticos canónicos de flujo:

- `ai flow violation in [@ai]: 'select' requires active session state (expected @sessionActive == true)`
- `ai flow violation in [@ai]: 'plan' requires model-ready state (expected @modelReady == true)`
- `ai flow violation in [@ai]: 'critique' requires planner-ready state (expected @plannerReady == true)`
- `ai flow violation in [@ai]: 'execute' requires critic-ready state (expected @criticReady == true)`
- `ai flow violation in [@ai]: 'complete' requires model-ready state (expected @modelReady == true)`

## Runtime observable

Estado runtime de ai:

- `ai.session.active`
- `ai.model.ready`
- `ai.planner.ready`
- `ai.critic.ready`
- `ai.executor.ready`
- `ai.memory.count`
- `ai.events.count`
- `ai.last_provider`
- `ai.last_model`
- `ai.last_goal`
- `ai.last_prompt`
- `ai.last_response`

Errores canónicos operativos:

- `ai.session.core.open requires non-empty session:str`
- `ai.session.core.close requires active session state`
- `ai.model.core.select requires non-empty provider:str`
- `ai.model.core.select requires non-empty model:str`
- `ai.model.core.select requires active session state`
- `ai.agent.core.plan requires non-empty goal:str`
- `ai.agent.core.plan requires model-ready state`
- `ai.agent.core.critique requires non-empty feedback:str`
- `ai.agent.core.critique requires planner-ready state`
- `ai.agent.core.execute requires non-empty task:str`
- `ai.agent.core.execute requires critic-ready state`
- `ai.chat.core.complete requires non-empty prompt:str`
- `ai.chat.core.complete requires model-ready state`
- `ai.memory.core.remember requires non-empty key:str`
- `ai.memory.core.remember requires non-empty value:str`
- `ai.memory.core.recall requires non-empty key:str`

Contrato inspect:

- paths `ai.*` requieren contrato de import runtime ai (`ai.session.core`, `ai.model.core`, `ai.agent.core`, `ai.chat.core`, `ai.memory.core`, o alias `nC.ai.*`).

## Evidencia de cierre

### Parser/Sema/IR

`crates/ncc-parser/src/lib.rs`:

- `ai_f19_tests::ai_at_operational_fixtures_parse_for_runtime`
- `ai_f19_tests::ai_at_invalid_section_fails_in_parser`

`crates/ncc-sema/src/lib.rs`:

- `ai_f19_sema_tests::ai_at_operational_fixtures_pass_in_sema`
- `ai_f19_sema_tests::ai_at_missing_baseline_stc_fails_in_sema`
- `ai_f19_sema_tests::ai_at_plan_without_model_fails_in_sema`
- `ai_f19_sema_tests::ai_at_execute_without_critique_fails_in_sema`
- `inspect_sema_tests::inspect_ai_runtime_path_without_ai_import_fails_in_sema`
- `inspect_sema_tests::inspect_ai_runtime_path_with_ai_import_passes_in_sema`

`crates/ncc-ir/src/lib.rs`:

- `ai_f19_ir_tests::lower_ai_fixture_lifecycle_into_typed_ir`

### Runtime/CLI

`crates/ncc-cli/src/main.rs`:

- `runtime_f19_ai_tests::runtime_ai_flow_updates_observable_state_and_inspect_paths`
- `runtime_f19_ai_tests::runtime_ai_complete_requires_non_empty_prompt`
- `runtime_f19_ai_fixture_matrix_tests::runtime_f19_check_ok_fixtures_run_successfully`
- `runtime_f19_ai_fixture_matrix_tests::runtime_f19_runtime_failures_are_observable_with_official_diagnostics`
- `runtime_f19_ai_fixture_matrix_tests::runtime_f19_check_fail_fixtures_stay_blocked_by_sema`

### Fixtures oficiales F19

`examples/ai_at/`:

- check/run OK:
  - `ai_at_lifecycle_ok.n`
  - `ai_at_alias_family_ok.n`
  - `ai_at_runtime_inspect_paths_ok.n`
- check OK, run FAIL canónico:
  - `ai_at_runtime_empty_prompt_fail.n`
- check FAIL canónico:
  - `ai_at_missing_baseline_stc_fail.n`
  - `ai_at_plan_without_model_fail.n`
  - `ai_at_execute_without_critique_fail.n`
  - `ai_at_invalid_section_fail.n` (bloqueado en parser por surface modal)

## Criterio binario de F19

F19 se considera `SI` cuando:

1. sesión/modelo/planner/critic/executor/memoria son ejecutables en runtime;
2. flujo semántico IA queda tipado y auditado en sema;
3. inspect IA expone estado operativo observable;
4. parser/sema/IR/runtime mantienen continuidad tipada sin fallback stringly;
5. existe matriz de fixtures `check/run` con OK/FAIL canónicos;
6. documentación de contratos queda publicada y enlazada en la matriz oficial v1.0.0.
