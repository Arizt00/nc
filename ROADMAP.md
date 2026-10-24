# nC roadmap

Work is organized around verifiable contracts. Dates will be assigned when a bounded, reproducible delivery exists.

## First-use experience

- Maintain native packages with hashes, matching runtimes and a tested first program.
- Complete native CLI help and document its differences from Stage0.
- Reduce provider-discovery steps while retaining explicit dependencies.
- Turn user reproductions into a small, stable regression corpus.

## Material continuity

- Consolidate typed instructions, terminators and control flow in `[@machine]`.
- Keep ABI, layout, linking and cache checks aligned with backend changes.
- Extend profiles through execution evidence, distinguishing them from layout-only validation.
- Document memory, pointers and regions according to the current manual contract.

## Bootstrap

- Preserve generation identity with identified sources and providers.
- Reduce Stage0-dependent routes without hidden substitutes.
- Define and verify the remaining global Stage3 promotion conditions.

## Community and documentation

- Keep manuals concise and examples executable.
- Turn recurring questions into precise documentation and diagnostics.
- Publish release notes with changes, limits and reproduction steps.

The [V81 status](docs/public/estado.md) records established scope. This roadmap describes planned work, not a support guarantee.
