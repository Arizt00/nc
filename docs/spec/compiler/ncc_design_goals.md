# Design Goals for `ncc`

Status: working design contract.
Scope: `ncc` as compiler, checker, executor, reporter and semantic authority for nC.

This document defines what `ncc` is trying to be. It is not a feature list.
It is a set of design goals that should guide compiler changes, reviews,
tests and release decisions.

## 1. Semantic Authority

`ncc` must be the official semantic authority for nC.

Editor tooling, formatters, LSPs, installers, package managers and external
adapters may expose or route information, but they must not invent language
semantics that contradict `ncc`.

Design consequences:

- diagnostics must originate from compiler-owned contracts;
- LSP and IDE integrations should consume compiler truth, not replace it;
- command-line reports should be structured enough for tools to depend on;
- semantic behavior must remain stable across presentation layers.

## 2. Modal Continuity

`ncc` must preserve the continuity of nC across:

- `[@]`;
- `[@machine]`;
- `primitive[@]`;
- linking;
- memory, pointers, regions and aliasing;
- runtime;
- higher modal layers.

No compiler change should treat these layers as isolated feature islands.
Parser, AST, sema, IR, lowering, backend, runtime and diagnostics must continue
to describe the same language.

## 3. Typed Internal Representation

`ncc` should prefer typed internal structures over stringly-typed shortcuts.

Design consequences:

- mode-specific syntax should become mode-specific AST when it has semantics;
- semantic categories should become enums or structured data where possible;
- IR must preserve enough typed information for lowering and diagnostics;
- backend/reporting text should be rendered from typed state, not used as the
  source of truth.

## 4. Phase Closure Discipline

A phase is not closed because syntax parses.

A compiler-facing feature is only ready when the relevant pieces align:

- lexer/parser behavior;
- AST shape;
- semantic validation;
- IR or NIR representation;
- lowering/backend/runtime behavior when applicable;
- positive and negative fixtures;
- regression tests;
- documentation of the decision.

This is the compiler version of the nC rule: nothing enters "half placed".

## 5. Clear Diagnostics

`ncc` must make invalid programs explainable.

Diagnostics should identify:

- what rule was violated;
- where the violation happened;
- which mode or contract owns the rule;
- whether the failure is syntactic, semantic, linking, runtime, backend or
  product/toolchain related.

Baseical diagnostics are part of the language surface. They should not be
treated as incidental CLI text.

## 6. Material Honesty

`ncc` must be honest about what is material, simulated, lowered, deferred or
externally delegated.

Design consequences:

- backend and runtime commands should say what they actually emitted or ran;
- external linkers, object converters and host tools must be visible in reports;
- primitive and machine evidence should distinguish parser closure from material
  execution;
- release documents should not promote deferred work as completed behavior.

## 7. Reproducibility

`ncc` should make validation reproducible from the repository.

Every important surface should have commands that can be run by a future
maintainer:

- `cargo test` coverage for compiler crates;
- fixture-driven CLI checks;
- JSON/report outputs for automation;
- stable examples for positive and negative behavior.

The default posture is: if a claim matters, it should have a command or fixture
that proves it.

## 8. Conservative Extensibility

`ncc` should be easy to extend without weakening existing contracts.

Design consequences:

- new modes should enter through explicit mode boundaries;
- new syntax should not bypass sema or IR;
- shared abstractions should be introduced only when they reduce real repeated
  complexity;
- existing typed paths should be reused before creating parallel behavior;
- compatibility decisions should be documented before they become accidental
  precedent.

## 9. Stable User Surface

`ncc` should expose a CLI that is predictable for humans and automations.

Goals:

- commands should have stable names and clear ownership;
- machine-readable output should be intentionally shaped;
- strict modes should be available when ambiguity matters;
- successful commands should not hide warnings that affect correctness;
- command behavior should map to compiler phases in a way users can understand.

## 10. Product Boundary

`ncc` compiles, checks, runs and reports nC. It should not become the whole
product manager.

Boundaries:

- `ncup` owns installation and toolchain lifecycle;
- `nfmt` owns formatting;
- `nlint` owns lint policy when separate from hard compiler errors;
- `nclsp` and `nc-sys-lsp` expose editor workflows;
- `ncc` remains the semantic and execution authority.

This keeps `ncc` strong without making it absorb every surrounding tool.

## 11. Performance Without Semantic Drift

Performance work must not change the meaning of nC.

Optimizers, native fast paths, backend shortcuts and runtime acceleration must:

- preserve parser and sema contracts;
- report which path produced the artifact;
- keep deoptimization or fallback behavior explicit;
- avoid introducing hidden language subsets unless documented as profiles.

Speed is valuable only when the compiler remains trustworthy.

## 12. Low-Level Respect

`ncc` must treat low-level modes as first-class language surfaces.

For `[@machine]` and `primitive[@]`, the compiler should preserve:

- explicit control flow;
- typed registers, operands and finalization;
- material hardware contracts;
- safety boundaries and responsibility boundaries;
- evidence that separates accepted syntax from executable reality.

Low-level does not mean loose. It means explicit.

## 13. Review Checklist

Before merging a meaningful `ncc` change, ask:

1. Does this preserve semantic authority in `ncc`?
2. Does this keep parser, AST, sema, IR and docs aligned?
3. Is any new behavior typed internally instead of parsed again from text?
4. Are positive and negative fixtures covered?
5. Are diagnostics clear and owned by the right mode or phase?
6. Does the CLI/reporting surface tell the truth about what happened?
7. Does this avoid duplicating semantics in tooling outside `ncc`?
8. Is any deferred behavior explicitly named as deferred?

## 14. Non-Goals

`ncc` should not:

- accept syntax just because it is pleasant to write;
- bypass semantic validation to reach backend output faster;
- duplicate runtime, linker or package-manager authority in ad hoc ways;
- let IDE or LSP behavior become the de facto language definition;
- treat generated reports as proof unless backed by tests or executable checks;
- reopen closed semantics silently during product or tooling work.

## 15. Source Documents

- [Base active](../../../_archive/deprecated/context_legacy/base/BASE_ACTIVE.md)
- [Roadmap master](../../../_archive/deprecated/context_legacy/roadmap/ROADMAP_MASTER.md)
- [Roadmap Codex master](../../handoff/ROADMAP_CODEX_MASTER.md)
- [Compiler pipeline](compiler_pipeline.md)
- [Product and toolchain base](../../base/producto_ncup_toolchain_bins.md)
- [nC table of contents](../../NC_TABLE_OF_CONTENTS.md)
