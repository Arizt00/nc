# nC v0.1 — Import/Use Family Diagnostics Matrix (Active Line)

## Scope
This matrix closes the import/use family coverage for the active line with fixture-backed sema validation.

## Baseical Diagnostic
Unknown import path must fail with:
- `unresolved import path '<path>'`

Context authority on declaration (when a container consumes import surface) must fail with:
- `invalid import declaration '<module.path[.*]>' for context '<container[@ctx]>': no compatible target context among [@ctx, ...]`

## Positive Family Fixture Matrix
| Family | Representative Imports | Fixture |
|---|---|---|
| Root.* | `Root.memory.region`, `Root.bind.core`, `Root.policy.capability` | `examples/import_use_families/import_use_family_root_ok.n` |
| Runtime.* | `Runtime.heap.region`, `Runtime.sync.core`, `Runtime.trace.core` | `examples/import_use_families/import_use_family_runtime_ok.n` |
| System.* | `System.net.core`, `System.socket.core`, `System.proc.core` | `examples/import_use_families/import_use_family_system_ok.n` |
| Machine.* | `Machine.reg`, `Machine.port`, `Machine.mmio` | `examples/import_use_families/import_use_family_machine_ok.n` |
| Security.* | `Security.tracekit`, `Security.hash`, `Security.policy` | `examples/import_use_families/import_use_family_security_ok.n` |
| Inspect.* | `Inspect.trace`, `Inspect.mem`, `Inspect.snapshot` | `examples/import_use_families/import_use_family_inspect_ok.n` |
| Compiler.* | `Compiler.parse.ast`, `Compiler.semantic.core`, `Compiler.backend.machine` | `examples/import_use_families/import_use_family_compiler_ok.n` |
| Tooling.* | `Tooling.fmt.core`, `Tooling.lint.core`, `Tooling.build.core`, `Tooling.pkg.core` | `examples/import_use_families/import_use_family_tooling_ok.n` |
| nC.list.* | `nC.list.*`, `nC.heap.core` | `examples/import_use_families/import_use_family_nc_list_ok.n` |
| nC.math.* | `nC.math.core`, `nC.math.algebra`, `nC.math.physics` | `examples/import_use_families/import_use_family_nc_math_ok.n` |
| nC.vector.* | `nC.vector.core`, `nC.vector.math` | `examples/import_use_families/import_use_family_nc_vector_ok.n` |
| nC.mem.* | `nC.mem.ptr`, `nC.mem.region` | `examples/import_use_families/import_use_family_nc_mem_ok.n` |
| nC.runtime.* bridge | `nC.runtime.heap`, `nC.runtime.net`, `nC.runtime.trace`, `nC.runtime.alias` | `examples/import_use_families/import_use_family_runtime_alias_bridge_ok.n` |
| Lower runtime surface | `system.net.core`, `memory.heap.manager`, `runtime.io` | `examples/import_use_families/import_use_family_lower_runtime_surface_ok.n` |
| `import` keyword parity | `import Root.policy.bind;`, `import System.net.core;`, `import nC.list.*;` | `examples/import_use_families/import_use_family_import_keyword_ok.n` |

## Negative Matrix (Unknown by Family)
These paths are required to fail with official unresolved-import diagnostics:
- `Root.memory.ghost`
- `Runtime.heap.ghost`
- `System.net.ghost`
- `Machine.ghost`
- `Security.ghost`
- `Inspect.ghost`
- `Compiler.parse.ghost`
- `Tooling.fmt.ghost`
- `nC.list.ghostFamily`
- `nC.math.ghost`
- `nC.vector.ghost`
- `nC.mem.ghost`

## Test Anchors
- `import_use_family_fixture_matrix_passes_in_sema`
- `import_use_family_unknown_fixture_reports_unresolved_import_path`
- `import_use_unknown_path_by_family_reports_official_diagnostic`
- `import_keyword_known_path_passes_in_sema`
- `import_keyword_unknown_path_reports_official_diagnostic`
- `import_declaration_context_authority_mismatch_fails_in_sema`
- `import_declaration_context_authority_runtime_wildcard_in_network_passes_in_sema`
- `import_declaration_context_authority_runtime_wildcard_in_system_fails_in_sema`

Location:
- `crates/ncc-sema/src/lib.rs` (module `linking_sema_tests`)

## Related Base
- `docs/spec/root/import_use_catalog_v0_1.md`
- `docs/spec/root/linking_diagnostics_official_matrix_v0_1.md`
