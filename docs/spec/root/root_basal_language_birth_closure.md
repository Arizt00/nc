# nC Root Basal Closure ([ @ ])

Status: active base (workspace line)
Scope: parser, sema, IR, examples, diagnostics, validation

## 1) Doctrinal role
`[@]` is the sovereign root context where language contracts are born: symbols, references, sections, pointer contracts, and entry/final rules.

## 2) Runtime policy in root
Current compiler line enforces strict root semantics for structural sections and pointer/strict checks, but runtime-family hard ban is still under migration across legacy root suites.

Operational rule in this line:
- root diagnostics MUST be explicit;
- no silent fallback on root pointer/nullability checks;
- print sensory remains forbidden outside `primitive[@]`.

## 3) Mode separation
- `[@]`: semantic sovereignty and authority contracts.
- `[@machine]`: machine/register/instruction surface.
- `primitive[@]`: low-level material/bridge surface.

## 4) Root sections
Root supports:
- `stc:`
- `binary:`
- `bind:`
- `raw:`
- `probe:`
- `fn:`
- `structure:`
- `to main` / `ref main` / `skk main`
- `fa:` / `final:`

Root invariants:
- `binary/bind/raw` only in root context.
- missing `fa/final` in root container is invalid.
- duplicate effective finals in same container are invalid.

## 5) References and linking
- `@symbol`, `@path`, `@fn(...)`, `@probe(...)` are typed in AST/sema.
- qualified references require resolvable and visible targets.
- ambiguity emits explicit diagnostics.

## 6) Memory root rules (implemented line)
- nullable pointer cannot be projected into non-nullable target.
- strict check (`!>`) is supported for pointer refinement.
- stale/invalid pointer diagnostics exist in memory/realloc flows.
- explicit region creation path is validated through typed calls (`regionFromPtr` family) and fixture gates.

## 7) Print family in root
Allowed root print surface includes structured modes already parsed and semantically checked.

Hard restriction in this line:
- `print.sensory(...)` is rejected outside `primitive[@]`.

## 8) Dev(error) root
Root flow supports explicit `dev(...)` contracts in statements and strict-check clauses.

## 9) Root IR contract
Lowering contains explicit typed root container IR:
- root section capture (`stc/binary/bind/raw/probe/fn/main/final`)
- root entry metadata
- root function/link metadata

## 10) Baseical examples (root)
- `examples/root/root_basic.nb`
- `examples/root/root_sections.nb`
- `examples/root/root_memory_arbiter.nb`
- `examples/root/root_imports.nb`
- `examples/root/root_references.nb`
- `examples/root/root_structure.nb`
- `examples/root/root_strictly.nb`
- `examples/root/root_bridge_machine.nb`
- `examples/root/root_bridge_primitive.nb`
- `examples/root/root_final.nb`
- `examples/root/root_print_structured.nb`
- `examples/root/root_forbidden_runtime.nb`
- `examples/root/root_forbidden_sensory.nb`

## 11) Validation commands
```bash
cargo fmt --all
cargo check --workspace --all-targets
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
```

Focused root checks:
```bash
cargo test --workspace -- --list | grep -i root
cargo test --workspace root -- --nocapture
```

Example checks:
```bash
ncc check examples/root/root_basic.nb
ncc check examples/root/root_sections.nb
ncc check examples/root/root_memory_arbiter.nb
```
