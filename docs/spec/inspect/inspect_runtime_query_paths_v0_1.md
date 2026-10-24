# nC v0.1 — F7 tranche B/C — official inspect runtime query paths

## Scope
This tranche hardens and expands semantic validation for inspect runtime query paths used in `print.inspect(...)`, preserving a typed path contract across parser -> sema -> runtime.
The active closure also includes import contracts and inspect-mode closure rules documented in:
- `docs/spec/inspect/inspect_mode_contracts_v0_1.md`

## Baseical paths in active line

Supported multi-segment inspect paths:
- `@heap.events.tail`
- `@heap.events.last`
- `@heap.events.count`
- `@heap.events.valid`
- `@heap.events.error`
- `@heap.live.allocs`
- `@heap.alloc.count`
- `@heap.blocks.count`
- `@net.connected`
- `@net.listening`
- `@net.accepted`
- `@net.last_send`
- `@net.host`
- `@net.port`
- `@net.last_payload`

Single-segment symbol inspect (`@symbol`) remains valid and uses regular symbol lookup.

## Runtime contract imports for official paths

Heap inspect paths require at least one runtime contract import:
- `use memory.heap.manager;` or
- `use stoud.memory { heap };`

Network inspect paths require at least one runtime contract import:
- `use system.net.core;` or
- `use alt.net.core;` or
- `use stoud.net { net };`

## Rejected paths

Any multi-segment inspect path outside the official set is rejected in sema with explicit diagnostic:

- `invalid inspect path '<path>': expected '<official inspect runtime path list>'`

Examples:
- `print.inspect(@heap.events.ghost);` -> reject
- `print.inspect(@heap.timeline.last);` -> reject
- `print.inspect(@net.ghost);` -> reject

## Why this tranche exists

- Avoid silent fallback to unknown/stringly runtime inspect routes.
- Preserve typed/explicit semantics between parser, sema and runtime.
- Keep inspect surface deterministic while broader `[@inspect]` mode remains in phased closure.
- Reuse a single typed inspect path mapping (`InspectRuntimePath`) across parser/sema/runtime, avoiding duplicated hardcoded lists.

## Test evidence

`crates/ncc-ast/src/lib.rs`:
- `inspect_runtime_path_ast_tests::inspect_runtime_path_from_segments_maps_official_paths`
- `inspect_runtime_path_ast_tests::inspect_runtime_path_from_segments_rejects_unknown_paths`

`crates/ncc-parser/src/lib.rs`:
- `root_f0_tests::root_at_inspect_runtime_official_paths_parse`

`crates/ncc-sema/src/lib.rs`:
- `inspect_sema_tests::inspect_heap_events_tail_ok_passes_in_sema`
- `inspect_sema_tests::inspect_heap_and_net_runtime_paths_ok_pass_in_sema`
- `inspect_sema_tests::inspect_unknown_path_fails_in_sema`
- `inspect_sema_tests::inspect_heap_path_without_contract_import_fails_in_sema`
- `inspect_sema_tests::inspect_net_path_without_contract_import_fails_in_sema`
- `inspect_sema_tests::inspect_context_final_print_enforces_runtime_contracts`
- `inspect_f7_fixture_sema_tests::*`

`crates/ncc-cli/src/main.rs`:
- `runtime_l2k_s8_inspect_fixture_matrix_tests::runtime_l2k_s8_check_ok_fixtures_run_successfully`
- `runtime_l2k_s8_inspect_fixture_matrix_tests::runtime_l2k_s8_inspect_runtime_paths_render_official_values`
