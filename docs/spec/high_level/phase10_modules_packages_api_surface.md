# High Level Phase 10 - Modules / Packages / Public API Surface

Phase 10 closes high-level code organization. It defines how modules are declared, how local packages group modules, how imports and exports are validated, and how public API promises are reported.

Closed surface:

- `module` declarations through file headers or `module:` sections.
- `package` declarations with name, version, profile and module membership.
- Exact imports, alias imports and profile-gated wildcard imports.
- Export and re-export declarations.
- `public`, `internal` and `private` visibility enforcement.
- Module graph construction and cycle diagnostics.
- Public API surface checks for `any`, private type leaks, unresolved bounds, async effects and Result/Option error exposure.

Out of scope:

- Ghor total.
- Package registry.
- External package manager.
- Linker integration.
- Closures, lambdas and captures.
- Reflection, compile-time evaluation, HKT and full specialization.

Next phase: High Level Closures, closures / lambdas / captures.
