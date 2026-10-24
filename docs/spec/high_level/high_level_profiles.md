# High Level Profiles

Status: Phase 0 audit baseline. Profiles are described for planning; no new profile semantics are implemented here.

## Baseline Profile

The current baseline accepts the existing high-level surface already exercised by examples and workspace tests:

- `stc:`, `fn:`, `fa:`
- `ref main ->> @...`
- imports/use families
- scalar and width-like types
- arrays, lists, `memRegion<T>`, `ptr<T>`
- control flow such as `if`, `while`, `return`
- compiler/runtime report surfaces

## Advanced Profile

The advanced profile should close modern syntax, advanced type rules, generics, contracts, collections, iterators, error model, and reflection without requiring the full expert risk model.

## Expert Profile

The expert profile permits dangerous operations when explicitly declared. It does not imply fake success. Every accepted risk must be representable, auditable, and lowerable or must fail with a diagnostic.

Reserved capability declarations include:

- `allowRawAccess`
- `allowUnsafeCast`
- `allowManualLifetime`
- `allowUncheckedIndex`
- `allowRace`
- `allowNoRollback`
- `allowPatchStacking`

## Dependency Baseline

Phase 0 confirms:

- Machine dependency: closed, `asm_complete_total=true`.
- Basal dependency: closed, `basal_phase7_release_seal_closed`.
- Compiler doctor: OK.
- Runtime doctor: OK.

High-level expert work may start only after the user explicitly approves High Level Fase 1.
