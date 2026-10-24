# nC v0.1 — F7 tranche C — inspect mode contracts

## Scope
This tranche closes `[@inspect]` in the active line as a typed modal surface, with parser/sema/runtime coherence and official diagnostics.

## `[@inspect]` container surface (active line)

Allowed sections:
- `stc`
- `fn`
- `probe`
- `final`

Rejected sections:
- `binary`
- `bind`
- `raw`
- `move fn`
- `structure`
- `to main` / `ref main` / `skk main`
- machine/primitive-only families

Baseical parser/sema diagnostic for rejected sections:
- `section not allowed in [@inspect] context`

## Structural contracts

- `[@inspect]` requires one `final` section.
- empty `final` is forbidden.

Baseical diagnostics:
- `missing final section in [@inspect] container`
- `empty final section in [@inspect] container`

## `print.inspect(...)` path contracts

Baseical runtime paths stay typed through `InspectRuntimePath` and are validated in sema.

For official heap inspect paths, sema requires one import contract:
- `use memory.heap.manager;`
- `use stoud.memory { heap };`

For official network inspect paths, sema requires one import contract:
- `use system.net.core;`
- `use alt.net.core;`
- `use stoud.net { net };`

Baseical diagnostics:
- `inspect path '<path>' requires heap runtime contract import: use memory.heap.manager; or use stoud.memory { heap };`
- `inspect path '<path>' requires network runtime contract import: use system.net.core; use alt.net.core; or use stoud.net { net };`

## Fixtures and tests

Fixtures:
- `examples/inspect_at/inspect_at_minimal_ok.n`
- `examples/inspect_at/inspect_at_runtime_contract_ok.n`
- `examples/inspect_at/inspect_at_heap_contract_missing_fail.n`
- `examples/inspect_at/inspect_at_net_contract_missing_fail.n`
- `examples/inspect_at/inspect_at_invalid_section_fail.n`
- `examples/inspect_at/inspect_at_main_entry_fail.n`

Test evidence:
- `crates/ncc-parser/src/lib.rs`
  - `inspect_f7_tests::*`
- `crates/ncc-sema/src/lib.rs`
  - `inspect_sema_tests::*`
  - `inspect_f7_fixture_sema_tests::*`

## Closure statement

F7 is closed in the active line for:
- typed inspect runtime paths,
- explicit inspect-mode modal restrictions,
- runtime import contracts for inspect heap/net surfaces,
- parser/sema/tests/docs aligned end-to-end.
