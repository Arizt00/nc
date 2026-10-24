# PSI migration roadmap

The delivered implementation is the strongest practical bridge that can be built on top of the existing text-based `NcSourceIndex`. It should not be the final semantic architecture.

## Phase 1: current package

- Stateless `GotoDeclarationHandler`.
- Project service.
- Cached file analysis.
- Scope-aware textual resolution.
- Indexed module lookup.
- Multiple navigation targets.

## Phase 2: typed PSI declarations

- Make declaration PSI nodes implement `PsiNameIdentifierOwner` or a project-specific named PSI interface.
- Preserve declaration kind, visibility, owner, and source range in PSI.
- Implement rename-safe `setName()`.

## Phase 3: PSI references

- Implement `PsiReference` for single-target identifiers.
- Implement `PsiPolyVariantReference` for overloaded or ambiguous imports.
- Move local, member, and module resolution into reference classes.
- Reuse the same resolver for completion and quick documentation.

## Phase 4: project index

- Add stubs for top-level containers, functions, types, constants, and public members.
- Add a `StubIndex` or `FileBasedIndex` keyed by symbol and module name.
- Remove filename-only search where semantic module identity is available.

## Phase 5: compiler/LSP parity

- Expose the ncc symbol graph through a stable protocol.
- Reconcile IDE resolution with compiler resolution.
- Use compiler diagnostics and semantic types for navigation, completion, inspections, and refactoring.

At the end of Phase 5, `NcGotoDeclarationHandler` becomes a small compatibility fallback or can be removed entirely because normal PSI reference resolution will drive Ctrl+B, Ctrl+click, Rename, Find Usages, and Quick Definition.
